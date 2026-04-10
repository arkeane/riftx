use std::cmp;
use std::collections::VecDeque;
use std::io::{self, ErrorKind, Read, Write};

use rayon::prelude::*;

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    ChaCha20Poly1305, Nonce,
    aead::{Aead, KeyInit},
};
use rand::random;
use zeroize::Zeroizing;

const SALT_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const FRAME_PLAINTEXT_LEN: usize = 64 * 1024;
const KEY_LEN: usize = 32;

// Argon2id parameters — kept separate from the frame size constant.
// 64 MB memory, 8 iterations, 1 thread. Higher t_cost is appropriate for an
// offline archiving tool where latency is not a constraint.
const ARGON2_M_COST: u32 = 65536; // 64 MB
const ARGON2_T_COST: u32 = 8;
const ARGON2_P_COST: u32 = 1;

/// Generates the 32-byte public salt
pub fn generate_salt() -> [u8; SALT_LEN] {
    random()
}

fn derive_key(password: &str, salt: &[u8; SALT_LEN]) -> io::Result<Zeroizing<[u8; KEY_LEN]>> {
    let params = Params::new(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST, Some(KEY_LEN))
        .map_err(|error| io::Error::new(ErrorKind::InvalidInput, error.to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    argon2
        .hash_password_into(password.as_bytes(), salt, key.as_mut())
        .map_err(|error| io::Error::new(ErrorKind::InvalidInput, error.to_string()))?;

    Ok(key)
}

/// Builds a 12-byte nonce from a u64 frame counter (LE) padded with zeros.
/// Counter nonces guarantee uniqueness without relying on RNG quality.
fn counter_nonce(frame_counter: u64) -> [u8; NONCE_LEN] {
    let mut nonce = [0u8; NONCE_LEN];
    nonce[..8].copy_from_slice(&frame_counter.to_le_bytes());
    nonce
}

/// The custom Russian Doll encryption layer.
///
/// On-disk frame format: `[plaintext_len: u32 LE] [ciphertext+tag: plaintext_len+TAG_LEN bytes]`
/// The nonce is derived from the frame counter and never stored on disk.
pub struct CryptoWriter<W: Write> {
    inner: Option<W>,
    cipher: ChaCha20Poly1305,
    /// Monotonically increasing counter; used as the per-frame nonce.
    frame_counter: u64,
}

impl<W: Write> CryptoWriter<W> {
    pub fn new(inner: W, password: &str, salt: &[u8; SALT_LEN]) -> io::Result<Self> {
        let key = derive_key(password, salt)?;
        let cipher = ChaCha20Poly1305::new_from_slice(key.as_ref())
            .map_err(|error| io::Error::new(ErrorKind::InvalidInput, error.to_string()))?;

        Ok(Self {
            inner: Some(inner),
            cipher,
            frame_counter: 0,
        })
    }

    pub fn finish(mut self) -> io::Result<W> {
        let mut inner = self
            .inner
            .take()
            .ok_or_else(|| io::Error::new(ErrorKind::BrokenPipe, "writer already finished"))?;
        inner.flush()?;
        Ok(inner)
    }

    fn write_frame(&mut self, plaintext: &[u8]) -> io::Result<()> {
        if plaintext.is_empty() {
            return Ok(());
        }

        let nonce_bytes = counter_nonce(self.frame_counter);
        self.frame_counter = self
            .frame_counter
            .checked_add(1)
            .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "frame counter overflow"))?;

        let ciphertext = self
            .cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
            .map_err(|error| io::Error::new(ErrorKind::InvalidData, error.to_string()))?;

        let plaintext_len = u32::try_from(plaintext.len())
            .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "frame too large"))?;

        let inner = self
            .inner
            .as_mut()
            .ok_or_else(|| io::Error::new(ErrorKind::BrokenPipe, "writer already finished"))?;

        inner.write_all(&plaintext_len.to_le_bytes())?;
        inner.write_all(&ciphertext)?;

        Ok(())
    }

    /// Encrypts multiple complete frames in parallel using rayon, then writes them in order.
    ///
    /// `buf` must be a multiple of `FRAME_PLAINTEXT_LEN` — the caller is responsible
    /// for splitting off any trailing partial frame before calling this.
    fn write_frames_parallel(&mut self, buf: &[u8]) -> io::Result<()> {
        debug_assert!(buf.len() % FRAME_PLAINTEXT_LEN == 0);

        let frames: Vec<&[u8]> = buf.chunks(FRAME_PLAINTEXT_LEN).collect();
        let n = frames.len() as u64;
        let base_counter = self.frame_counter;

        // Verify the counter can accommodate all frames before doing any work.
        base_counter
            .checked_add(n)
            .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "frame counter overflow"))?;

        // Encrypt all frames in parallel. `ChaCha20Poly1305::encrypt` takes `&self`
        // so the cipher is `Sync` and safe to share across rayon threads.
        let results: Vec<io::Result<(u32, Vec<u8>)>> = {
            let cipher = &self.cipher;
            frames
                .par_iter()
                .enumerate()
                .map(|(i, frame)| {
                    let nonce_bytes = counter_nonce(base_counter + i as u64);
                    let plaintext_len = u32::try_from(frame.len())
                        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "frame too large"))?;
                    let ciphertext = cipher
                        .encrypt(Nonce::from_slice(&nonce_bytes), *frame)
                        .map_err(|e| io::Error::new(ErrorKind::InvalidData, e.to_string()))?;
                    Ok((plaintext_len, ciphertext))
                })
                .collect()
        };

        let inner = self
            .inner
            .as_mut()
            .ok_or_else(|| io::Error::new(ErrorKind::BrokenPipe, "writer already finished"))?;

        for result in results {
            let (plaintext_len, ciphertext) = result?;
            inner.write_all(&plaintext_len.to_le_bytes())?;
            inner.write_all(&ciphertext)?;
        }

        self.frame_counter += n;
        Ok(())
    }
}

impl<W: Write> Write for CryptoWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        // If there are 2+ complete frames available, encrypt them in parallel.
        // The 2-frame threshold avoids rayon overhead on small or trickle writes.
        let complete_len = (buf.len() / FRAME_PLAINTEXT_LEN) * FRAME_PLAINTEXT_LEN;
        if complete_len >= 2 * FRAME_PLAINTEXT_LEN {
            self.write_frames_parallel(&buf[..complete_len])?;
            if complete_len < buf.len() {
                self.write_frame(&buf[complete_len..])?;
            }
        } else {
            let mut offset = 0;
            while offset < buf.len() {
                let end = cmp::min(offset + FRAME_PLAINTEXT_LEN, buf.len());
                self.write_frame(&buf[offset..end])?;
                offset = end;
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner
            .as_mut()
            .ok_or_else(|| io::Error::new(ErrorKind::BrokenPipe, "writer already finished"))?
            .flush()
    }
}

pub struct CryptoReader<R: Read> {
    inner: R,
    cipher: ChaCha20Poly1305,
    /// Frame counter used to reconstruct the nonce; must advance in lockstep with the writer.
    frame_counter: u64,
    /// Holds decrypted bytes that the upper layers haven't consumed yet.
    internal_buffer: VecDeque<u8>,
}

impl<R: Read> CryptoReader<R> {
    pub fn new(inner: R, password: &str, salt: &[u8; SALT_LEN]) -> io::Result<Self> {
        let key = derive_key(password, salt)?;
        let cipher = ChaCha20Poly1305::new_from_slice(key.as_ref())
            .map_err(|error| io::Error::new(ErrorKind::InvalidInput, error.to_string()))?;

        Ok(Self {
            inner,
            cipher,
            frame_counter: 0,
            internal_buffer: VecDeque::new(),
        })
    }

    fn fill_internal_buffer(&mut self) -> io::Result<bool> {
        let mut plaintext_len_bytes = [0u8; 4];
        let mut filled = 0;

        while filled < plaintext_len_bytes.len() {
            let bytes_read = self.inner.read(&mut plaintext_len_bytes[filled..])?;
            if bytes_read == 0 {
                if filled == 0 {
                    return Ok(false);
                }

                return Err(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "truncated encrypted frame header",
                ));
            }

            filled += bytes_read;
        }

        let plaintext_len = u32::from_le_bytes(plaintext_len_bytes) as usize;
        if plaintext_len > FRAME_PLAINTEXT_LEN {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "encrypted frame exceeds maximum size",
            ));
        }

        let nonce_bytes = counter_nonce(self.frame_counter);
        self.frame_counter = self
            .frame_counter
            .checked_add(1)
            .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "frame counter overflow"))?;

        let ciphertext_len = plaintext_len.checked_add(TAG_LEN).ok_or_else(|| {
            io::Error::new(ErrorKind::InvalidData, "invalid encrypted frame size")
        })?;
        let mut ciphertext = vec![0u8; ciphertext_len];
        self.inner.read_exact(&mut ciphertext)?;

        let plaintext = self
            .cipher
            .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
            .map_err(|error| io::Error::new(ErrorKind::InvalidData, error.to_string()))?;

        self.internal_buffer.extend(plaintext);
        Ok(true)
    }

    /// Drains up to `buf.len()` bytes from the internal buffer into `buf`.
    fn consume_buffer(&mut self, buf: &mut [u8]) -> usize {
        let to_read = cmp::min(buf.len(), self.internal_buffer.len());
        for (dst, src) in buf[..to_read]
            .iter_mut()
            .zip(self.internal_buffer.drain(..to_read))
        {
            *dst = src;
        }
        to_read
    }
}

impl<R: Read> Read for CryptoReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        if self.internal_buffer.is_empty() && !self.fill_internal_buffer()? {
            return Ok(0);
        }

        Ok(self.consume_buffer(buf))
    }
}

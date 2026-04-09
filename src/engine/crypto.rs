use std::cmp;
use std::io::{self, ErrorKind, Read, Write};

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::random;

const SALT_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const FRAME_PLAINTEXT_LEN: usize = 64 * 1024;
const KEY_LEN: usize = 32;

/// Generates the 32-byte public salt
pub fn generate_salt() -> [u8; 32] {
    random()
}

fn derive_key(password: &str, salt: &[u8; SALT_LEN]) -> io::Result<[u8; KEY_LEN]> {
    let params = Params::new(64 * 1024, 3, 1, Some(KEY_LEN))
        .map_err(|error| io::Error::new(ErrorKind::InvalidInput, error.to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key = [0u8; KEY_LEN];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|error| io::Error::new(ErrorKind::InvalidInput, error.to_string()))?;

    Ok(key)
}

/// The custom Russian Doll encryption layer
pub struct CryptoWriter<W: Write> {
    inner: Option<W>,
    cipher: ChaCha20Poly1305,
}

impl<W: Write> CryptoWriter<W> {
    pub fn new(inner: W, password: &str, salt: &[u8; SALT_LEN]) -> io::Result<Self> {
        let key = derive_key(password, salt)?;
        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|error| io::Error::new(ErrorKind::InvalidInput, error.to_string()))?;

        Ok(Self {
            inner: Some(inner),
            cipher,
        })
    }

    pub fn finish(mut self) -> io::Result<W> {
        let mut inner = self.inner.take().ok_or_else(|| {
            io::Error::new(ErrorKind::BrokenPipe, "writer already finished")
        })?;
        inner.flush()?;
        Ok(inner)
    }

    fn write_frame(&mut self, plaintext: &[u8]) -> io::Result<()> {
        if plaintext.is_empty() {
            return Ok(());
        }

        let nonce_bytes = random::<[u8; NONCE_LEN]>();
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
        inner.write_all(&nonce_bytes)?;
        inner.write_all(&ciphertext)?;

        Ok(())
    }
}

impl<W: Write> Write for CryptoWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let mut offset = 0;
        while offset < buf.len() {
            let end = cmp::min(offset + FRAME_PLAINTEXT_LEN, buf.len());
            self.write_frame(&buf[offset..end])?;
            offset = end;
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
    
    /// Holds decrypted bytes that the upper layers haven't asked for yet
    internal_buffer: Vec<u8>,
}

impl<R: Read> CryptoReader<R> {
    pub fn new(inner: R, password: &str, salt: &[u8; SALT_LEN]) -> io::Result<Self> {
        let key = derive_key(password, salt)?;
        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|error| io::Error::new(ErrorKind::InvalidInput, error.to_string()))?;

        Ok(Self {
            inner,
            cipher,
            internal_buffer: Vec::new(),
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

        let mut nonce_bytes = [0u8; NONCE_LEN];
        self.inner.read_exact(&mut nonce_bytes)?;

        let ciphertext_len = plaintext_len
            .checked_add(TAG_LEN)
            .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "invalid encrypted frame size"))?;
        let mut ciphertext = vec![0u8; ciphertext_len];
        self.inner.read_exact(&mut ciphertext)?;

        let plaintext = self
            .cipher
            .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
            .map_err(|error| io::Error::new(ErrorKind::InvalidData, error.to_string()))?;

        self.internal_buffer.extend_from_slice(&plaintext);
        Ok(true)
    }
}

impl<R: Read> Read for CryptoReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        if !self.internal_buffer.is_empty() {
            let to_read = cmp::min(buf.len(), self.internal_buffer.len());
            buf[..to_read].copy_from_slice(&self.internal_buffer[..to_read]);
            self.internal_buffer.drain(..to_read);

            return Ok(to_read);
        }

        if !self.fill_internal_buffer()? {
            return Ok(0);
        }

        let to_read = cmp::min(buf.len(), self.internal_buffer.len());
        buf[..to_read].copy_from_slice(&self.internal_buffer[..to_read]);
        self.internal_buffer.drain(..to_read);

        Ok(to_read)
    }
}
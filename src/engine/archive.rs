use crate::engine::crypto::{CryptoReader, CryptoWriter, generate_salt};
use crate::engine::utils::{ByteCounter, init_progress_bar};
use indicatif::{ProgressBar, ProgressStyle};
use liblzma::read::XzDecoder;
use liblzma::stream::MtStreamBuilder;
use liblzma::write::XzEncoder;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, ErrorKind, Read, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;
use tar::{Archive, Builder};

fn get_thread_count() -> u32 {
    thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1)
}

fn cleanup_failed_unpack(dir: &Path, pb: &ProgressBar, err: io::Error) -> Box<dyn Error> {
    if dir.exists() {
        if let Err(e) = fs::remove_dir_all(dir) {
            eprintln!(
                "warning: failed to remove partial extraction at '{}': {}",
                dir.display(),
                e
            );
        }
    }
    pb.abandon_with_message("Failed.");
    err.into()
}

pub fn pack(
    src: &Path,
    dest: &Path,
    password: Option<&str>,
    noenc: bool,
) -> Result<(), Box<dyn Error>> {
    if !src.is_dir() {
        return Err(
            io::Error::new(ErrorKind::InvalidInput, "Error: source must be a directory").into(),
        );
    }

    let pb = init_progress_bar();
    pb.enable_steady_tick(Duration::from_millis(80));

    let mut file = File::create(dest)?;

    // 1. Optional Encryption Layer
    let writer: Box<dyn Write> = if !noenc && let Some(pass) = password {
        let salt = generate_salt();
        file.write_all(&salt)?;
        pb.set_message("Deriving key...");
        Box::new(CryptoWriter::new(file, pass, &salt)?)
    } else {
        Box::new(file)
    };

    // 2. Compression Layer
    let mt_stream = MtStreamBuilder::new()
        .threads(get_thread_count())
        .preset(6)
        .encoder()?;
    let compressor = XzEncoder::new_stream(writer, mt_stream);

    // 3. Tar Layer
    pb.set_message("Packing...");
    let mut tar_builder = Builder::new(ByteCounter::with_progress(compressor, pb.clone()));
    tar_builder.append_dir_all(".", src)?;

    // 4. Finalize (unwrapping the stack)
    let byte_counter = tar_builder.into_inner()?;
    let final_src = byte_counter.count();
    let compressor = byte_counter.into_inner();
    let mut inner_writer = compressor.finish()?;
    inner_writer.flush()?;

    let final_out = fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
    pb.finish_with_message(format!(
        "Done. {:.1} MB → {:.1} MB",
        final_src as f64 / 1_048_576.0,
        final_out as f64 / 1_048_576.0,
    ));

    Ok(())
}

pub fn unpack(
    src: &Path,
    dest: &Path,
    password: Option<&str>,
    noenc: bool,
) -> Result<(), Box<dyn Error>> {
    if dest.exists() {
        return Err(io::Error::new(ErrorKind::AlreadyExists, "destination already exists").into());
    }

    let pb = init_progress_bar();
    pb.enable_steady_tick(Duration::from_millis(80));
    let mut file = File::open(src)?;

    // 1. Optional Decryption Layer
    let reader: Box<dyn Read> = if !noenc && let Some(pass) = password {
        let mut salt = [0u8; 32];
        file.read_exact(&mut salt)?;
        pb.set_message("Deriving key...");
        let encrypted_len = fs::metadata(src)?.len().saturating_sub(32);
        pb.set_length(encrypted_len);
        Box::new(CryptoReader::new(pb.wrap_read(file), pass, &salt)?)
    } else {
        pb.set_length(fs::metadata(src)?.len());
        Box::new(file)
    };

    pb.set_style(
        ProgressStyle::with_template("{spinner} {msg} {bytes}/{total_bytes}")
            .unwrap()
            .tick_strings(&["⣾", "⣷", "⣯", "⣟", "⣻", "⣽", "⣾", "⣷"]),
    );
    pb.set_message("Unpacking...");

    // 2. Decompression & Tar
    let decompressor = XzDecoder::new(reader);
    let mut tar_archive = Archive::new(decompressor);

    if let Err(e) = tar_archive.unpack(dest) {
        return Err(cleanup_failed_unpack(dest, &pb, e));
    }

    pb.finish_with_message("Done.");
    Ok(())
}

use crate::engine::crypto::{CryptoReader, CryptoWriter, generate_salt};
use crate::engine::utils::{ByteCounter, init_progress_bar};
use indicatif::ProgressStyle;
use liblzma::read::XzDecoder;
use liblzma::stream::MtStreamBuilder;
use liblzma::write::XzEncoder;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;
use tar::Archive;
use tar::Builder;

pub fn pack(
    source_dir: &Path,
    destination_file: &Path,
    password: &str,
) -> Result<(), Box<dyn Error>> {
    if !source_dir.is_dir() {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            "pack source must be a directory",
        )
        .into());
    }

    let pb = init_progress_bar();
    pb.enable_steady_tick(Duration::from_millis(80));

    // 1. Core: Open the file
    let mut dest_file = File::create(destination_file)?;

    // 2. Write the plain-text salt to the very top of the file FIRST
    let salt = generate_salt();
    dest_file.write_all(&salt)?;

    // 3. Layer 1: The Encryptor
    pb.set_message("Deriving key...");
    let encryptor = CryptoWriter::new(dest_file, password, &salt)?;

    // 4. Layer 2: The Compressor (multi-threaded)
    // TODO: MtStreamBuilder blocks XzEncoder::write() at block boundaries while
    // worker threads sync, which stalls ByteCounter and makes the progress bar
    // freeze at reproducible positions. This should be investigated and fixed
    // As of now the progress bar is usefull enough to show overall progress.
    let thread_count = thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1);

    let mt_stream = MtStreamBuilder::new()
        .threads(thread_count)
        .preset(6)
        .encoder()?;

    let compressor = XzEncoder::new_stream(encryptor, mt_stream);

    // 5. The Shell: The Tar Builder — ByteCounter sits between tar and the
    //    compressor, counting source bytes and updating the spinner live.
    let mut tar_builder = Builder::new(ByteCounter::with_progress(compressor, pb.clone()));

    // 6. Execute the streaming push
    tar_builder.append_dir_all(".", source_dir)?;

    // 7. Gracefully dismantle and finalize the pipeline
    let byte_counter = tar_builder.into_inner()?;
    let final_src = byte_counter.count();
    let compressor = byte_counter.into_inner();
    let encryptor = compressor.finish()?;
    encryptor.finish()?;

    let final_out = fs::metadata(destination_file).map(|m| m.len()).unwrap_or(0);
    pb.finish_with_message(format!(
        "Done. {:.1} MB → {:.1} MB",
        final_src as f64 / 1_048_576.0,
        final_out as f64 / 1_048_576.0,
    ));

    Ok(())
}

pub fn unpack(
    source_file: &Path,
    destination_dir: &Path,
    password: &str,
) -> Result<(), Box<dyn Error>> {
    if destination_dir.exists() {
        return Err(std::io::Error::new(
            ErrorKind::AlreadyExists,
            "destination folder already exists",
        )
        .into());
    }

    let pb = init_progress_bar();
    pb.enable_steady_tick(Duration::from_millis(80));

    // File size minus 32-byte salt header — used for bytes-based progress during extraction
    let encrypted_len = fs::metadata(source_file)?.len().saturating_sub(32);

    let mut src_file = File::open(source_file)?;

    // 1. Read the first 32 bytes to get the salt
    let mut salt = [0u8; 32];
    src_file.read_exact(&mut salt)?;

    // 2. Layer 1: The Decryptor
    pb.set_message("Deriving key...");
    let decryptor = CryptoReader::new(pb.wrap_read(src_file), password, &salt)?;

    pb.set_length(encrypted_len);
    pb.set_style(
        ProgressStyle::with_template("{spinner} {msg} [{bytes}/{total_bytes}]")
            .unwrap()
            .tick_strings(&["⣾", "⣷", "⣯", "⣟", "⣻", "⣽", "⣾", "⣷"]),
    );
    pb.set_message("Unpacking...");

    // 3. Layer 2: The Decompressor
    let decompressor = XzDecoder::new(decryptor);

    // 4. The Shell: The Tar Archive
    let mut tar_archive = Archive::new(decompressor);

    // 5. Execute the streaming pull
    if let Err(error) = tar_archive.unpack(destination_dir) {
        // Always attempt to remove a partially-extracted destination on any error
        // so the caller is never left with incomplete, potentially-decrypted data.
        if destination_dir.exists()
            && let Err(cleanup_err) = fs::remove_dir_all(destination_dir)
        {
            eprintln!(
                "warning: failed to remove partial extraction at '{}': {}",
                destination_dir.display(),
                cleanup_err
            );
        }

        pb.abandon_with_message("Failed.");
        return Err(error.into());
    }

    pb.finish_with_message("Done.");
    Ok(())
}

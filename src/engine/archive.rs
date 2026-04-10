use crate::engine::crypto::{CryptoReader, CryptoWriter, generate_salt};
use liblzma::read::XzDecoder;
use liblzma::write::XzEncoder;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::path::Path;
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

    // 1. Core: Open the file
    let mut dest_file = File::create(destination_file)?;

    // 2. Write the plain-text salt to the very top of the file FIRST
    let salt = generate_salt();
    dest_file.write_all(&salt)?;

    // 3. Layer 1: The Encryptor
    let encryptor = CryptoWriter::new(dest_file, password, &salt)?;

    // 4. Layer 2: The Compressor
    let compressor = XzEncoder::new(encryptor, 6);

    // 5. The Shell: The Tar Builder
    let mut tar_builder = Builder::new(compressor);

    // 6. Execute the streaming push
    tar_builder.append_dir_all(".", source_dir)?;

    // 7. Gracefully dismantle and finalize the pipeline
    let compressor = tar_builder.into_inner()?;
    let encryptor = compressor.finish()?;
    encryptor.finish()?;

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

    let mut src_file = File::open(source_file)?;

    // 1. Read the first 32 bytes to get the salt
    let mut salt = [0u8; 32];
    src_file.read_exact(&mut salt)?;

    // 2. Layer 1: The Decryptor
    let decryptor = CryptoReader::new(src_file, password, &salt)?;

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

        return Err(error.into());
    }

    Ok(())
}

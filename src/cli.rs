use std::error::Error;
use std::io::{self};
use std::path::{Path, PathBuf};

use rpassword::prompt_password;

pub fn prompt_for_password(provided_password: Option<&str>) -> Result<String, Box<dyn Error>> {
    if let Some(password) = provided_password {
        return Ok(password.to_owned());
    }

    Ok(prompt_password("Password: ")?)
}

pub fn prompt_for_password_with_confirmation(provided_password: Option<&str>) -> Result<String, Box<dyn Error>> {
    if let Some(password) = provided_password {
        return Ok(password.to_owned());
    }

    let password = prompt_password("Password: ")?;
    let confirmation = prompt_password("Confirm password: ")?;

    if password != confirmation {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "password confirmation did not match",
        )
        .into());
    }

    if password.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "password cannot be empty",
        )
        .into());
    }

    Ok(password)
}

pub fn default_unpack_output(input_path: &Path) -> PathBuf {
    let archive_name = input_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("archive");

    let folder_name = archive_name
        .strip_suffix(".tar.xz.enc")
        .or_else(|| archive_name.strip_suffix(".enc"))
        .unwrap_or(archive_name);

    let output_name = if folder_name.is_empty() {
        "archive"
    } else {
        folder_name
    };

    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(output_name)
}
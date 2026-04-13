use clap::{CommandFactory, Parser, Subcommand};
use std::error::Error;
use std::path::PathBuf;

mod cli;

use riftx::{pack, unpack};

use cli::*;

#[derive(Parser)]
#[command(
    name = "riftx",
    version,
    about = "Securely pack and unpack project folders using ChaCha20-Poly1305 encryption",
    long_about = "RiftX is a high-performance CLI tool for creating encrypted archives. \
                  It pipelines tar archiving, xz compression, and ChaCha20-Poly1305 \
                  encryption to ensure your data remains private and compact.",
    after_help = riftx::disclaimer().unwrap()
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create an encrypted .riftx archive from a directory
    #[command(
        visible_alias = "p",
        after_help = "Password resolution order: --password flag > RIFTX_PASSWORD env var > interactive prompt.\n\
                      WARNING: --password exposes the secret in process listings and shell history."
    )]
    Pack {
        /// Source directory to archive
        #[arg(short, long, value_name = "DIR")]
        input: String,

        /// Output file path [default: <INPUT>.riftx]
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,

        /// Encryption password (omit for secure prompt).
        /// WARNING: passing a password via this flag exposes it in process
        /// listings and shell history. Prefer the interactive prompt.
        #[arg(short, long, value_name = "STR")]
        password: Option<String>,
    },

    /// Extract and decrypt a .riftx archive
    #[command(
        visible_alias = "u",
        after_help = "Password resolution order: --password flag > RIFTX_PASSWORD env var > interactive prompt.\n\
                      WARNING: --password exposes the secret in process listings and shell history."
    )]
    Unpack {
        /// Encrypted archive to extract
        #[arg(short, long, value_name = "FILE")]
        input: String,

        /// Destination directory [default: current folder]
        #[arg(short, long, value_name = "DIR")]
        output: Option<String>,

        /// Decryption password (omit for secure prompt).
        /// WARNING: passing a password via this flag exposes it in process
        /// listings and shell history. Prefer the interactive prompt.
        #[arg(short, long, value_name = "STR")]
        password: Option<String>,
    },
}

fn main() {
    if let Err(error) = run_cli() {
        eprintln!("{}", error);
        std::process::exit(1);
    }
}

fn run_cli() -> std::result::Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Pack {
            input,
            output,
            password,
        }) => {
            let input_path = std::path::Path::new(input);
            let output_path = output.as_ref().map(PathBuf::from).unwrap_or_else(|| {
                let archive_name = input_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("archive");

                PathBuf::from(format!("{}.riftx", archive_name))
            });
            let password = prompt_for_password_with_confirmation(password.as_deref())?;

            pack(input_path, &output_path, password.as_str())?;
        }
        Some(Commands::Unpack {
            input,
            output,
            password,
        }) => {
            let input_path = std::path::Path::new(input);
            let output_path = output
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| default_unpack_output(input_path));
            let password = prompt_for_password(password.as_deref())?;

            unpack(input_path, &output_path, password.as_str())?;
        }
        None => {
            // No subcommand provided, show help
            Cli::command().print_help()?;
            println!();
        }
    }

    Ok(())
}

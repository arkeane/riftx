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
    long_about = None,
    after_help = "Copyright (c) 2026, Ludovico Pestarino. Use at your own risk."
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

        /// Disable Encryption (creates a standard .tar.xz)
        #[arg(long = "no-enc", value_name = "BOOL")]
        noenc: bool,
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

        /// Treat the archive as unencrypted
        #[arg(long = "no-enc", value_name = "BOOL")]
        noenc: bool,
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
            noenc,
        }) => {
            let input_path = std::path::Path::new(input);
            let output_path = output.as_ref().map(PathBuf::from).unwrap_or_else(|| {
                // .with_extension("riftx") replaces any existing extension or adds it if missing
                input_path.with_extension("riftx")
            });
            let password = prompt_for_password_with_confirmation(password.as_deref())?;

            if *noenc {
                pack(input_path, &output_path, None, *noenc)?;
            } else {
                let password = prompt_for_password_with_confirmation(password.as_deref())?;
                pack(input_path, &output_path, Some(password.as_str()), *noenc)?;
            }
        }
        Some(Commands::Unpack {
            input,
            output,
            password,
            noenc,
        }) => {
            let input_path = std::path::Path::new(input);
            let output_path = output
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| default_unpack_output(input_path));
            let password = prompt_for_password(password.as_deref())?;

            if *noenc {
                unpack(input_path, &output_path, None, *noenc)?;
            } else {
                let password = prompt_for_password(password.as_deref())?;
                unpack(input_path, &output_path, Some(password.as_str()), *noenc)?;
            }
        }
        None => {
            // No subcommand provided, show help
            Cli::command().print_help()?;
            println!();
        }
    }

    Ok(())
}

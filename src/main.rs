use std::error::Error;
use std::path::PathBuf;
use clap::{Parser, Subcommand};

mod cli;
mod tui;

use riftx::{pack, unpack};

use cli::*;
use tui::*;

#[derive(Parser)]
#[command(name = "riftx", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(name = "pack", about = "pack a directory into a .tar.xz.enc file")]
    Pack {
        #[arg(short, long, help = "the directory to pack")]
        input: String,
        #[arg(short, long, help = "the output file, defaults to <dir_name>.tar.xz.enc", required = false)]
        output: Option<String>,
        #[arg(short, long, help = "the password or key", required = false)]
        password: Option<String>,
    },

    #[command(name = "unpack", about = "unpack a .tar.xz.enc file into a directory")]
    Unpack {
        #[arg(short, long, help = "the file to unpack")]
        input: String,
        #[arg(short, long, help = "the output directory, defaults to <folder_name> in the current working folder", required = false)]
        output: Option<String>,
        #[arg(short, long, help = "the password or key", required = false)]
        password: Option<String>,
    }
}

fn main(){
    if let Err(error) = run_cli() {
        eprintln!("{}", error);
        std::process::exit(1);
    }
}

fn run_cli() -> std::result::Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Pack { input, output, password }) => {
            let input_path = std::path::Path::new(input);
            let output_path = output
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    let archive_name = input_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("archive");

                    PathBuf::from(format!("{}.tar.xz.enc", archive_name))
                });
            let password = prompt_for_password_with_confirmation(password.as_deref())?;

            pack(input_path, &output_path, password.as_str())?;
        }
        Some(Commands::Unpack { input, output, password }) => {
            let input_path = std::path::Path::new(input);
            let output_path = output
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| default_unpack_output(input_path));
            let password = prompt_for_password(password.as_deref())?;

            unpack(input_path, &output_path, password.as_str())?;
        }
        None => {
            run_tui();
        }
    }

    Ok(())
}
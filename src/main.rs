use crossterm::event::{self, Event};
use ratatui::{DefaultTerminal, Frame};
use std::error::Error;
use std::io::Result as IoResult;
use std::path::{Path, PathBuf};
use clap::{Parser, Subcommand};

use riftx::{pack, unpack};

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
        #[arg(short, long, help = "the output file, defaults to <dir_name>.tar.xz.enc")]
        output: Option<String>,
        #[arg(short, long, help = "the password or key")]
        password: String,
    },

    #[command(name = "unpack", about = "unpack a .tar.xz.enc file into a directory")]
    Unpack {
        #[arg(short, long, help = "the file to unpack")]
        input: String,
        #[arg(short, long, help = "the output directory, defaults to <folder_name> in the current working folder")]
        output: Option<String>,
        #[arg(short, long, help = "the password or key")]
        password: String,
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

            pack(input_path, &output_path, password.as_str())?;
        }
        Some(Commands::Unpack { input, output, password }) => {
            let input_path = std::path::Path::new(input);
            let output_path = output
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| default_unpack_output(input_path));

            unpack(input_path, &output_path, password.as_str())?;
        }
        None => {
            run_ui();
        }
    }

    Ok(())
}

fn default_unpack_output(input_path: &Path) -> PathBuf {
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


// Basic TUI example, to be replaced with the actual UI later
fn run_ui() {
    let terminal = ratatui::init();
    let _result = run(terminal);
    ratatui::restore();
}

fn run(mut terminal: DefaultTerminal) -> IoResult<()> {
    loop {
        terminal.draw(render)?;
        if matches!(event::read()?, Event::Key(_)) {
            break Ok(());
        }
    }
}

fn render(frame: &mut Frame) {
    frame.render_widget("hello world", frame.area());
}
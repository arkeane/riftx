use crossterm::event::{self, Event};
use ratatui::{DefaultTerminal, Frame};
use std::io::Result;
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
        #[arg(short, long, help = "the output file")]
        output: String,
        #[arg(short, long, help = "the password or key")]
        password: String,
    },

    #[command(name = "unpack", about = "unpack a .tar.xz.enc file into a directory")]
    Unpack {
        #[arg(short, long, help = "the file to unpack")]
        input: String,
        #[arg(short, long, help = "the output directory")]
        output: String,
        #[arg(short, long, help = "the password or key")]
        password: String,
    }
}

fn main(){
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Pack { input, output, password }) => {
            let input_path = std::path::Path::new(input);
            let output_path = std::path::Path::new(output);

            pack(input_path, output_path, password.as_str());
        }
        Some(Commands::Unpack { input, output, password }) => {
            let input_path = std::path::Path::new(input);
            let output_path = std::path::Path::new(output);

            unpack(input_path, output_path, password.as_str());
        }
        None => {
            run_ui();
        }
    }
}


// Basic TUI example, to be replaced with the actual UI later
fn run_ui() {
    let terminal = ratatui::init();
    let _result = run(terminal);
    ratatui::restore();
}

fn run(mut terminal: DefaultTerminal) -> Result<()> {
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
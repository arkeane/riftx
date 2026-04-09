use crossterm::event::{self, Event};
use ratatui::{DefaultTerminal, Frame};
use std::io::{Result as IoResult};

// Basic TUI example, to be replaced with the actual UI later
pub fn run_tui() {
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
use indicatif::ProgressBar;
use std::io::{self, IsTerminal, Write, stdout};

const UPDATE_INTERVAL: u64 = 256 * 1024;

// Wraps any writer and counts the total bytes that pass through it.
// Used to measure source bytes fed into the compression pipeline.
// Used in `pack` to update the progress bar message with live throughput information.
// When constructed with `with_progress`, updates the progress bar message
// every 256 KB so the user sees live throughput during `pack`.
pub struct ByteCounter<W: Write> {
    inner: W,
    pb: ProgressBar,
    count: u64,
    next_update: u64,
}

impl<W: Write> ByteCounter<W> {
    pub fn with_progress(inner: W, pb: ProgressBar) -> Self {
        Self {
            inner,
            pb,
            count: 0,
            next_update: 0,
        }
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Write> Write for ByteCounter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.count += n as u64;
        if self.count >= self.next_update {
            self.next_update = self.count + UPDATE_INTERVAL;
            self.pb.set_message(format!(
                "Packing... {:.1} MB",
                self.count as f64 / 1_048_576.0,
            ));
        }
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

// Initializes a progress bar if stdout is a terminal, otherwise returns a hidden progress bar.
pub fn init_progress_bar() -> ProgressBar {
    let pb = if stdout().is_terminal() {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            indicatif::ProgressStyle::with_template("{spinner} {msg}")
                .unwrap()
                .tick_strings(&["⣾", "⣷", "⣯", "⣟", "⣻", "⣽", "⣾", "⣷"]),
        );
        pb
    } else {
        ProgressBar::hidden()
    };

    pb
}

pub fn disclaimer() -> Result<String, ()> {
    let disclaimer = format!(
        r#"Copyright (c) 2026, Ludovico Pestarino
This tool is a work in progress and may not be fully functional or accurate.
Use at your own risk.
The author of this tool does not take any responsibility for the accuracy or completeness of the data provided by this tool."#,
    );

    Ok(disclaimer)
}

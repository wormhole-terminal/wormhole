//! The readiness pipe the app hands the host on descriptor 3.

use std::fs::File;
use std::io::Write as _;

use super::unix::ready as os;

/// The write end of the readiness pipe: one line, `ok <pid>` or `err <text>`, then closed.
pub(crate) struct ReadyPipe(Option<File>);

impl ReadyPipe {
    /// Take the pipe if the app passed one, and keep it from the program the host starts.
    pub(crate) fn take() -> Self {
        Self(os::take())
    }

    /// Whether the line was said already (or there is no pipe).
    pub(crate) fn is_spent(&self) -> bool {
        self.0.is_none()
    }

    /// Say `line` (one line, without its newline) and close the pipe. Only the first call says
    /// anything.
    pub(crate) fn say(&mut self, line: &str) {
        if let Some(mut file) = self.0.take() {
            let line: String = line.chars().map(|c| if c == '\n' || c == '\r' { ' ' } else { c }).collect();
            let _ = file.write_all(format!("{line}\n").as_bytes());
        }
    }
}

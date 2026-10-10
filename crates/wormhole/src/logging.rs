//! Logging: a daily-rotated file in `<data dir>/logs/` (kept for a week),
//! plus stderr when launched from a terminal.
//!
//! Level via `WORMHOLE_LOG` (e.g. `WORMHOLE_LOG=debug`), default `info`.

use std::io::IsTerminal;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};

/// Where the log files are (Help → Show Logs in Finder opens it).
pub fn dir() -> std::path::PathBuf {
    wormhole_core::store::data_dir().join("logs")
}

/// Install the global subscriber. Keep the guard alive for the whole run so
/// buffered lines are flushed on exit.
pub fn init() -> Option<WorkerGuard> {
    let filter = EnvFilter::try_from_env("WORMHOLE_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let stderr = std::io::stderr().is_terminal().then(|| fmt::layer().with_writer(std::io::stderr));

    let dir = dir();
    // The appender prunes old files on start and complains if the folder is missing.
    let _ = std::fs::create_dir_all(&dir);
    let appender = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("wormhole")
        .filename_suffix("log")
        .max_log_files(7)
        .build(&dir);
    let (file, guard) = match appender {
        Ok(appender) => {
            let (writer, guard) = tracing_appender::non_blocking(appender);
            (Some(fmt::layer().with_ansi(false).with_writer(writer)), Some(guard))
        }
        Err(_) => (None, None),
    };

    tracing_subscriber::registry().with(filter).with(stderr).with(file).init();
    guard
}

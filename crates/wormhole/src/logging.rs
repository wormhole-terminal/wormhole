//! Logging: a daily-rotated file in `<data dir>/logs/` (kept for a week),
//! plus stderr when launched from a terminal.
//!
//! Level via `WORMHOLE_LOG` (e.g. `WORMHOLE_LOG=debug`), default `info`.
//!
//! A panic is written down too ([`install_panic_hook`]): launched from the Finder, the app has no
//! stderr anyone reads, so without it a release build's panic left nothing but the crash report.

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

/// The file the last panic is written to, beside the logs.
const LAST_PANIC: &str = "last-panic.txt";

/// Write every panic to the log and, synchronously, to `logs/last-panic.txt`, then run the
/// default hook (stderr). The log's writer is a background thread that an abort doesn't wait
/// for, which is why the file is written here before the hook returns.
pub fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let report = panic_report(
            thread.name().unwrap_or("unnamed"),
            &panic_message(info.payload()),
            &info.location().map(ToString::to_string).unwrap_or_default(),
            &std::backtrace::Backtrace::force_capture().to_string(),
        );
        tracing::error!("{report}");
        let path = dir().join(LAST_PANIC);
        // Nothing to do if this fails: the default hook below still prints it.
        let _ = wormhole_core::fsutil::write_atomic(
            &path,
            report.as_bytes(),
            wormhole_core::fsutil::LinkPolicy::Follow,
        );
        default(info);
    }));
}

/// What a panic was raised with: `panic!`'s text, or a placeholder for any other payload.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "(a panic without a message)".to_string()
    }
}

fn panic_report(thread: &str, message: &str, location: &str, backtrace: &str) -> String {
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default();
    format!(
        "Wormhole {version} panicked on thread '{thread}' at {location} (unix time {at}):\n{message}\n\n{backtrace}",
        version = env!("CARGO_PKG_VERSION"),
    )
}

#[cfg(test)]
mod tests {
    use super::{panic_message, panic_report};

    #[test]
    fn a_panic_message_is_its_text_whatever_the_payload() {
        assert_eq!(panic_message(&"boom"), "boom");
        assert_eq!(panic_message(&String::from("boom")), "boom");
        assert_eq!(panic_message(&42_u8), "(a panic without a message)");
    }

    #[test]
    fn the_report_names_the_version_thread_place_and_message() {
        let report = panic_report("main", "index out of bounds", "src/x.rs:3:5", "0: frame");
        assert!(report.starts_with(&format!(
            "Wormhole {} panicked on thread 'main' at src/x.rs:3:5",
            env!("CARGO_PKG_VERSION")
        )));
        assert!(report.contains("\nindex out of bounds\n"));
        assert!(report.ends_with("0: frame"));
    }
}

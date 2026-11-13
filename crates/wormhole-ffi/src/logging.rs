//! Logging for the Swift app, as the GPUI app's `app/logging.rs` does it: a daily file in `<data dir>/logs/` (seven kept),
//! the level from `WORMHOLE_LOG` (default `info`), stderr too when launched from a terminal, and a panic written to
//! `logs/last-panic.txt`. The data folder is core's (`WORMHOLE_DATA_DIR` of a test instance), so a test instance logs into
//! its own folder. The Swift side's own lines come in through [`log_line`].

use std::io::IsTerminal;
use std::sync::OnceLock;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};

/// The file the last panic is written to, beside the logs.
const LAST_PANIC: &str = "last-panic.txt";

/// Kept for the whole run so buffered lines are flushed on exit.
static GUARD: OnceLock<Option<WorkerGuard>> = OnceLock::new();

fn dir() -> std::path::PathBuf {
    wormhole_core::storage::paths::data_dir().join("logs")
}

/// Install the subscriber and the panic hook, once, at launch (a second call does nothing). Returns the logs folder.
#[uniffi::export]
pub fn logging_start() -> String {
    GUARD.get_or_init(|| {
        let guard = init();
        install_panic_hook();
        guard
    });
    dir().display().to_string()
}

/// One line from the Swift side into the same files; `level` is `error`, `warn`, `info` or `debug`.
#[uniffi::export]
pub fn log_line(level: String, message: String) {
    match level.as_str() {
        "error" => tracing::error!(target: "swift", "{message}"),
        "warn" => tracing::warn!(target: "swift", "{message}"),
        "debug" => tracing::debug!(target: "swift", "{message}"),
        _ => tracing::info!(target: "swift", "{message}"),
    }
}

fn init() -> Option<WorkerGuard> {
    let filter = EnvFilter::try_from_env("WORMHOLE_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let stderr = std::io::stderr().is_terminal().then(|| fmt::layer().with_writer(std::io::stderr));

    let dir = dir();
    // The appender prunes old files on start and complains if the folder is missing; owner-only, as it makes its
    // files with the umask's mode, so the folder is what keeps them private.
    if wormhole_core::storage::files::ensure_private_dir(&dir).is_err() {
        let _ = std::fs::create_dir_all(&dir);
    }
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
    // A subscriber installed already (a test) is left in place.
    let _ = tracing_subscriber::registry().with(filter).with(stderr).with(file).try_init();
    guard
}

/// Every panic goes to the log and, synchronously, to `logs/last-panic.txt` (the log's writer is a background thread an
/// abort does not wait for), then the default hook runs.
fn install_panic_hook() {
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
        let _ = wormhole_core::storage::files::write_atomic(
            &dir().join(LAST_PANIC),
            report.as_bytes(),
            wormhole_core::storage::files::LinkPolicy::Follow,
        );
        default(info);
    }));
}

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

/// The data folder's files an earlier version made with the umask's modes become owner-only (core's rule; the GPUI app
/// runs it on a thread of its own at launch, so this does too and never holds up the launch).
#[uniffi::export]
pub fn make_data_dir_private() {
    let _ = std::thread::Builder::new().name("private-data".into()).spawn(|| {
        wormhole_core::storage::store::make_data_dir_private(&wormhole_core::storage::paths::data_dir());
    });
}

//! The host's log: `<data dir>/logs/host-<id>.log`, one line per event, `0600`. What is logged is
//! sizes, ids and states, never terminal content or the environment (section 9).
//!
//! A subscriber of its own, a few dozen lines, instead of `tracing-subscriber` and what it pulls
//! in: the host stays small, and it needs nothing but lines in a file.

use std::fmt::{self, Write as _};
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Metadata, Subscriber};
use wormhole_term::sys::fs::{private_dir, private_file};

/// A log that grew past this is started over when the host starts.
const MAX_SIZE: u64 = 1024 * 1024;

/// Where the host with `id` logs.
pub fn path(data_dir: &Path, id: &str) -> PathBuf {
    data_dir.join("logs").join(format!("host-{id}.log"))
}

/// The level `WORMHOLE_LOG` names (as the app reads it), `info` otherwise.
pub fn level_from(value: Option<&str>) -> Level {
    match value.map(str::to_ascii_lowercase).as_deref() {
        Some("trace") => Level::TRACE,
        Some("debug") => Level::DEBUG,
        Some("warn") => Level::WARN,
        Some("error") => Level::ERROR,
        _ => Level::INFO,
    }
}

/// Log to `path` from now on. Without a file to write (a read-only data dir) the host runs
/// without a log.
pub fn init(path: &Path, level: Level) {
    let Some(dir) = path.parent() else { return };
    if private_dir(DirBuilder::new().recursive(true)).create(dir).is_err() {
        return;
    }
    let too_big = fs::metadata(path).is_ok_and(|m| m.len() > MAX_SIZE);
    let mut options = OpenOptions::new();
    private_file(options.create(true));
    if too_big {
        options.write(true).truncate(true);
    } else {
        options.append(true);
    }
    let Ok(file) = options.open(path) else { return };
    let _ = tracing::subscriber::set_global_default(FileLog { file: Mutex::new(file), max: level });
}

struct FileLog {
    file: Mutex<fs::File>,
    max: Level,
}

/// One event's fields as text: the message, then `name=value` for the rest.
#[derive(Default)]
struct Line {
    message: String,
    fields: String,
}

impl Visit for Line {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
        } else {
            let _ = write!(self.fields, " {}={value:?}", field.name());
        }
    }
}

impl Subscriber for FileLog {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        *metadata.level() <= self.max
    }

    fn max_level_hint(&self) -> Option<tracing::level_filters::LevelFilter> {
        Some(tracing::level_filters::LevelFilter::from_level(self.max))
    }

    // Nothing in the host's code opens spans; the ones a dependency opens are not recorded.
    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut line = Line::default();
        event.record(&mut line);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        let text = format!(
            "{}.{:03} {} pid={} {}: {}{}\n",
            now.as_secs(),
            now.subsec_millis(),
            event.metadata().level(),
            std::process::id(),
            event.metadata().target(),
            line.message,
            line.fields
        );
        if let Ok(mut file) = self.file.lock() {
            let _ = file.write_all(text.as_bytes());
        }
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_read_as_the_app_reads_them() {
        assert_eq!(level_from(Some("DEBUG")), Level::DEBUG);
        assert_eq!(level_from(Some("nonsense")), Level::INFO);
        assert_eq!(level_from(None), Level::INFO);
    }

    #[test]
    fn the_log_lives_in_the_data_dirs_logs() {
        assert_eq!(path(Path::new("/data"), "abcdefghij12"), Path::new("/data/logs/host-abcdefghij12.log"));
    }
}

//! The meta file of a session host: `<hosts dir>/<id>.meta`, how the app, core and the host itself
//! find out which host is which without asking it (and, with `Welcome`, whether it is the one they
//! think it is: never by executable path or by pid alone).
//!
//! UTF-8 text, one `key=value` per line, unknown keys ignored by readers, `0600`, written
//! atomically (a temporary file in the same directory, renamed over).

use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{self, Read as _, Write as _};
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};

/// The file format this build writes and reads.
pub const FORMAT: u32 = 1;
/// The most a meta file may hold when read (they are a few hundred bytes).
pub const READ_MAX: u64 = 16 * 1024;

/// Whether `id` is a host id: 12 characters from `[a-z0-9]`.
pub fn valid_id(id: &str) -> bool {
    id.len() == 12 && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

/// What a host says about itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Meta {
    pub id: String,
    /// The host's process id.
    pub pid: u32,
    /// The shell's, once known.
    pub child_pid: Option<u32>,
    /// Unix seconds.
    pub started_at: u64,
    pub host_version: String,
    pub proto_min: u16,
    pub proto_max: u16,
    /// Where the host listens (absolute).
    pub socket: PathBuf,
    /// The app's data directory the host belongs to (absolute).
    pub data_dir: PathBuf,
}

/// Why a meta file could not be used.
#[derive(Debug)]
pub enum MetaError {
    Io(io::Error),
    /// A line is not `key=value` or a value is not what its key needs.
    Malformed(&'static str),
    /// A key every reader needs is missing.
    Missing(&'static str),
    /// Written by a build that knows a newer format.
    NewerFormat(u32),
}

impl std::fmt::Display for MetaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => e.fmt(f),
            Self::Malformed(key) => write!(f, "meta: bad value for {key}"),
            Self::Missing(key) => write!(f, "meta: {key} is missing"),
            Self::NewerFormat(n) => write!(f, "meta: format {n} is newer than this build reads"),
        }
    }
}

impl std::error::Error for MetaError {}

impl From<io::Error> for MetaError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// `<dir>/<id>.meta`.
pub fn path_in(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.meta"))
}

/// Read and parse the meta file at `path`; see [`Meta::read`].
pub fn read(path: &Path) -> Result<Meta, MetaError> {
    Meta::read(path)
}

/// Write `meta` to `<dir>/<id>.meta` atomically, `0600`; see [`Meta::write`].
pub fn write(dir: &Path, meta: &Meta) -> io::Result<PathBuf> {
    meta.write(dir)
}

/// Remove `<dir>/<id>.meta`; a file that is not there is fine.
pub fn remove(dir: &Path, id: &str) {
    Meta::remove(dir, id);
}

impl Meta {
    /// The file's text. Fails when a value would break the line format.
    pub fn render(&self) -> io::Result<String> {
        let socket = self.socket.to_str();
        let data_dir = self.data_dir.to_str();
        let (Some(socket), Some(data_dir)) = (socket, data_dir) else {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "meta paths must be UTF-8"));
        };
        if [self.id.as_str(), self.host_version.as_str(), socket, data_dir].iter().any(|v| v.contains('\n')) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "meta values are one line"));
        }
        let mut text = String::new();
        let _ = writeln!(text, "format={FORMAT}");
        let _ = writeln!(text, "id={}", self.id);
        let _ = writeln!(text, "pid={}", self.pid);
        if let Some(child) = self.child_pid {
            let _ = writeln!(text, "child_pid={child}");
        }
        let _ = writeln!(text, "started_at={}", self.started_at);
        let _ = writeln!(text, "host_version={}", self.host_version);
        let _ = writeln!(text, "proto_min={}", self.proto_min);
        let _ = writeln!(text, "proto_max={}", self.proto_max);
        let _ = writeln!(text, "socket={socket}");
        let _ = writeln!(text, "data_dir={data_dir}");
        Ok(text)
    }

    /// Read a meta file's text. Unknown keys are skipped; a repeated key takes its last value.
    pub fn parse(text: &str) -> Result<Self, MetaError> {
        let (mut format, mut id, mut pid, mut child_pid, mut started_at) = (None, None, None, None, None);
        let (mut version, mut proto_min, mut proto_max, mut socket, mut data_dir) =
            (None, None, None, None, None);
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else { continue };
            match key {
                "format" => format = Some(value.parse::<u32>().map_err(|_| MetaError::Malformed("format"))?),
                "id" => id = Some(value.to_owned()),
                "pid" => pid = Some(value.parse::<u32>().map_err(|_| MetaError::Malformed("pid"))?),
                "child_pid" => {
                    child_pid = Some(value.parse::<u32>().map_err(|_| MetaError::Malformed("child_pid"))?);
                }
                "started_at" => {
                    started_at = Some(value.parse::<u64>().map_err(|_| MetaError::Malformed("started_at"))?);
                }
                "host_version" => version = Some(value.to_owned()),
                "proto_min" => {
                    proto_min = Some(value.parse::<u16>().map_err(|_| MetaError::Malformed("proto_min"))?);
                }
                "proto_max" => {
                    proto_max = Some(value.parse::<u16>().map_err(|_| MetaError::Malformed("proto_max"))?);
                }
                "socket" => socket = Some(PathBuf::from(value)),
                "data_dir" => data_dir = Some(PathBuf::from(value)),
                _ => {}
            }
        }
        let format = format.ok_or(MetaError::Missing("format"))?;
        if format > FORMAT {
            return Err(MetaError::NewerFormat(format));
        }
        let id = id.ok_or(MetaError::Missing("id"))?;
        if !valid_id(&id) {
            return Err(MetaError::Malformed("id"));
        }
        Ok(Self {
            id,
            pid: pid.ok_or(MetaError::Missing("pid"))?,
            child_pid,
            started_at: started_at.unwrap_or(0),
            host_version: version.unwrap_or_default(),
            proto_min: proto_min.unwrap_or(1),
            proto_max: proto_max.unwrap_or(1),
            socket: socket.ok_or(MetaError::Missing("socket"))?,
            data_dir: data_dir.unwrap_or_default(),
        })
    }

    /// Write `<dir>/<id>.meta`: a `0600` temporary file in `dir`, renamed over the old one, so a
    /// reader sees the whole old file or the whole new one. Returns the path.
    pub fn write(&self, dir: &Path) -> io::Result<PathBuf> {
        let text = self.render()?;
        if !valid_id(&self.id) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a host id"));
        }
        let path = path_in(dir, &self.id);
        let temp = dir.join(format!(".{}.meta.{}", self.id, std::process::id()));
        // A leftover of a crashed write by a process with this pid is replaced.
        let _ = fs::remove_file(&temp);
        let mut file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(&temp)?;
        let written = file.write_all(text.as_bytes()).and_then(|()| file.flush());
        drop(file);
        let renamed = written.and_then(|()| fs::rename(&temp, &path));
        if renamed.is_err() {
            let _ = fs::remove_file(&temp);
        }
        renamed?;
        Ok(path)
    }

    /// Read and parse the meta file at `path` (at most [`READ_MAX`] bytes of it).
    pub fn read(path: &Path) -> Result<Self, MetaError> {
        let mut text = String::new();
        fs::File::open(path)?.take(READ_MAX).read_to_string(&mut text)?;
        Self::parse(&text)
    }

    /// Remove `<dir>/<id>.meta`; a file that is not there is fine.
    pub fn remove(dir: &Path, id: &str) {
        let _ = fs::remove_file(path_in(dir, id));
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt as _;

    use super::*;

    fn sample() -> Meta {
        Meta {
            id: "abcdefghij12".into(),
            pid: 4242,
            child_pid: Some(4243),
            started_at: 1_760_000_000,
            host_version: "1.2.3".into(),
            proto_min: 1,
            proto_max: 1,
            socket: "/Users/me/Library/Application Support/Wormhole/hosts/abcdefghij12.sock".into(),
            data_dir: "/Users/me/Library/Application Support/Wormhole".into(),
        }
    }

    #[test]
    fn a_meta_file_round_trips_through_its_text() {
        let meta = sample();
        let text = meta.render().expect("renders");
        assert!(text.starts_with("format=1\nid=abcdefghij12\npid=4242\nchild_pid=4243\n"), "{text}");
        assert_eq!(Meta::parse(&text).expect("parses"), meta);
    }

    #[test]
    fn unknown_keys_and_junk_lines_are_ignored_and_a_newer_format_is_refused() {
        let mut text = sample().render().expect("renders");
        text.push_str("future_key=whatever\nnot a pair\n");
        assert_eq!(Meta::parse(&text).expect("parses"), sample());
        let newer = text.replace("format=1", "format=2");
        assert!(matches!(Meta::parse(&newer), Err(MetaError::NewerFormat(2))));
        let no_pid: String =
            text.lines().filter(|l| !l.starts_with("pid=")).map(|l| format!("{l}\n")).collect();
        assert!(matches!(Meta::parse(&no_pid), Err(MetaError::Missing("pid"))));
        assert!(matches!(
            Meta::parse(&text.replace("id=abcdefghij12", "id=../x")),
            Err(MetaError::Malformed("id"))
        ));
    }

    #[test]
    fn a_value_with_a_newline_is_not_written() {
        let mut meta = sample();
        meta.socket = "/tmp/a\nb".into();
        assert!(meta.render().is_err());
    }

    #[test]
    fn writing_is_atomic_private_and_replaces_the_old_file() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let meta = sample();
        let path = write(dir.path(), &meta).expect("writes");
        assert_eq!(path, dir.path().join("abcdefghij12.meta"));
        assert_eq!(fs::metadata(&path).expect("exists").permissions().mode() & 0o777, 0o600);
        let mut later = meta.clone();
        later.child_pid = Some(9);
        later.write(dir.path()).expect("replaces");
        assert_eq!(read(&path).expect("reads").child_pid, Some(9));
        let names: Vec<_> =
            fs::read_dir(dir.path()).expect("lists").map(|e| e.expect("entry").file_name()).collect();
        assert_eq!(names.len(), 1, "no temporary file is left: {names:?}");
        Meta::remove(dir.path(), "abcdefghij12");
        assert!(!path.exists());
        Meta::remove(dir.path(), "abcdefghij12");
    }

    #[test]
    fn ids_are_twelve_lowercase_letters_and_digits() {
        assert!(valid_id("abcdefghij12"));
        for bad in ["", "abcdefghij1", "abcdefghij123", "ABCDEFGHIJ12", "abcdefghij1/", "abcdefghij1é"] {
            assert!(!valid_id(bad), "{bad}");
        }
    }
}

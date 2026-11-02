//! The report as plain English text, for a bug report: the header and one block a host. The window
//! has its own words (the catalogs); this text is for pasting, like the logs and the command line,
//! and stays English.

use std::fmt::Write as _;

use wormhole_term::client::LinkMode;
use wormhole_wire::version::HostFit;

use super::{Answer, AppFacts, HostRow, Report, Standing};
use crate::ipc::hosts::StaleReason;

impl Report {
    /// The text a bug report carries: the header, then every row.
    pub fn to_text(&self) -> String {
        let counts = self.counts();
        let mut out = String::new();
        let _ = writeln!(
            out,
            "Wormhole {} (protocol {} (minor {}))",
            self.app.version, self.app.proto_major, self.app.proto_minor
        );
        let _ = writeln!(out, "hosts folder: {}", self.hosts_dir.display());
        let _ = writeln!(out, "persistent terminals: {}", if self.flag_on { "on" } else { "off" });
        let binary = self.host_binary.as_deref().map_or("not found".to_string(), |p| p.display().to_string());
        let _ = writeln!(out, "host binary: {binary}");
        let _ = writeln!(
            out,
            "hosts: {} live ({} attached, {} detached, {} orphan), {} stale",
            counts.live(),
            counts.attached,
            counts.detached,
            counts.orphan,
            counts.stale
        );
        for row in &self.rows {
            out.push('\n');
            out.push_str(&row.to_text(&self.app));
        }
        out
    }
}

impl HostRow {
    /// This row as text, one fact a line.
    pub fn to_text(&self, app: &AppFacts) -> String {
        let mut out = String::new();
        let mut line = |key: &str, value: String| {
            let _ = writeln!(out, "  {key}: {value}");
        };
        let unknown = || "unknown".to_string();
        line("state", standing_word(&self.standing));
        line("host pid", self.host_pid.map_or_else(unknown, |p| p.to_string()));
        line("shell pid", self.shell_pid.map_or_else(unknown, |p| p.to_string()));
        line("program", self.program.clone().unwrap_or_else(unknown));
        line("folder", self.cwd.as_ref().map_or_else(unknown, |p| p.display().to_string()));
        line(
            "started",
            self.started_at.map_or_else(unknown, |t| {
                let up = self.uptime_secs.map(|u| format!(", up {}", duration_words(u))).unwrap_or_default();
                format!("{}{up}", time_words(t))
            }),
        );
        let mismatch =
            if self.version_differs(app) { format!(" (this app is {})", app.version) } else { String::new() };
        line("host version", format!("{}{mismatch}", self.host_version.clone().unwrap_or_else(unknown)));
        line(
            "protocol",
            match (self.proto, self.fit) {
                (Some((min, max)), Some(fit)) => format!("{}, {}", range_words(min, max), fit_word(fit)),
                (Some((min, max)), None) => format!("{}, not running", range_words(min, max)),
                _ => unknown(),
            },
        );
        line("socket", self.socket.as_ref().map_or_else(unknown, |p| p.display().to_string()));
        line("meta", self.meta_path.display().to_string());
        line("answers", answer_word(&self.answer));
        line("memory", self.rss.map_or_else(unknown, bytes_words));
        if let Some(u) = &self.unavailable {
            line("terminal", format!("cannot show it: {}", u.detail));
        }
        if let Some(l) = &self.link {
            let flag = |on: bool, word: &str| if on { format!(", {word}") } else { String::new() };
            line(
                "link",
                format!(
                    "{}, {} B in / {} B out, {} frames in / {} out{}{}{}",
                    mode_word(l.mode),
                    l.bytes_in,
                    l.bytes_out,
                    l.frames_in,
                    l.frames_out,
                    flag(l.detached, "detached"),
                    flag(l.lost, "lost"),
                    flag(l.ended, "ended"),
                ),
            );
        }
        format!("[{}]\n{out}", self.id)
    }
}

fn standing_word(standing: &Standing) -> String {
    match standing {
        Standing::Attached { window, title } => format!("attached (window {window}, \"{title}\")"),
        Standing::Detached => "detached".to_string(),
        Standing::Orphan => "orphan (no terminal names it)".to_string(),
        Standing::Stale(reason) => format!("stale ({})", stale_word(reason)),
    }
}

fn answer_word(answer: &Answer) -> String {
    match answer {
        Answer::NotAsked => "not asked".to_string(),
        Answer::Yes(i) => format!(
            "yes (host {}, protocol {}.{}, {}, {}x{}, {} history lines, revision {}{})",
            i.version,
            i.proto,
            i.proto_minor,
            if i.exited { "program ended" } else { "program running" },
            i.cols,
            i.rows,
            i.history_lines,
            i.revision,
            if i.alt_screen { ", alternate screen" } else { "" }
        ),
        Answer::Incompatible => "yes, but speaks no protocol this build does".to_string(),
        Answer::No(why) => format!("no ({why})"),
    }
}

/// Why a host is stale, in English.
pub fn stale_word(reason: &StaleReason) -> String {
    match reason {
        StaleReason::Unreadable(why) => format!("meta unreadable: {why}"),
        StaleReason::PidGone => "its process is gone".to_string(),
        StaleReason::SocketMissing => "no socket where the meta says".to_string(),
        StaleReason::NoMeta => "a socket with no meta file".to_string(),
    }
}

fn mode_word(mode: LinkMode) -> &'static str {
    match mode {
        LinkMode::Headless => "headless",
        LinkMode::Raw => "raw",
        LinkMode::Sync => "sync",
    }
}

fn fit_word(fit: HostFit) -> String {
    match fit {
        HostFit::Compatible(major) => format!("compatible (major {major})"),
        HostFit::TooOld => "too old for this app".to_string(),
        HostFit::TooNew => "too new for this app".to_string(),
    }
}

/// A host's range of protocol majors in words: `1`, or `1 to 2`.
pub fn range_words(min: u16, max: u16) -> String {
    if min == max { min.to_string() } else { format!("{min} to {max}") }
}

/// A length of time as the two units a person reads: `12s`, `7m 30s`, `5h 12m`, `3d 4h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elapsed {
    Seconds(u64),
    MinutesSeconds(u64, u64),
    HoursMinutes(u64, u64),
    DaysHours(u64, u64),
}

/// `secs` split into [`Elapsed`]'s two largest units.
pub fn elapsed(secs: u64) -> Elapsed {
    let (d, h, m, s) = (secs / 86_400, secs % 86_400 / 3600, secs % 3600 / 60, secs % 60);
    match (d, h, m) {
        (0, 0, 0) => Elapsed::Seconds(s),
        (0, 0, _) => Elapsed::MinutesSeconds(m, s),
        (0, _, _) => Elapsed::HoursMinutes(h, m),
        _ => Elapsed::DaysHours(d, h),
    }
}

/// [`elapsed`] in English: `3d 4h`, `5h 12m`, `7m 30s`, `12s`.
pub fn duration_words(secs: u64) -> String {
    match elapsed(secs) {
        Elapsed::Seconds(s) => format!("{s}s"),
        Elapsed::MinutesSeconds(m, s) => format!("{m}m {s}s"),
        Elapsed::HoursMinutes(h, m) => format!("{h}h {m}m"),
        Elapsed::DaysHours(d, h) => format!("{d}d {h}h"),
    }
}

/// The unit of a size from [`size`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeUnit {
    Bytes,
    Kib,
    Mib,
}

/// `bytes` as a number and a binary unit: whole bytes below 1 KiB, whole KiB below 1 MiB, then MiB
/// with one digit (`40.5`).
pub fn size(bytes: u64) -> (SizeUnit, String) {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * KIB;
    if bytes >= MIB {
        (SizeUnit::Mib, format!("{:.1}", bytes as f64 / MIB as f64))
    } else if bytes >= KIB {
        (SizeUnit::Kib, (bytes / KIB).to_string())
    } else {
        (SizeUnit::Bytes, bytes.to_string())
    }
}

/// [`size`] in English: `512 B`, `300 KiB`, `40.5 MiB`.
pub fn bytes_words(bytes: u64) -> String {
    let (unit, number) = size(bytes);
    let unit = match unit {
        SizeUnit::Bytes => "B",
        SizeUnit::Kib => "KiB",
        SizeUnit::Mib => "MiB",
    };
    format!("{number} {unit}")
}

/// A moment in UTC as a date (`2026-10-30`) and a time (`14:03:07`), for Unix seconds; `None` for
/// one no calendar has.
pub fn utc_parts(secs: u64) -> Option<(String, String)> {
    let moment = i64::try_from(secs).ok().and_then(|s| chrono::DateTime::from_timestamp(s, 0))?;
    Some((moment.format("%Y-%m-%d").to_string(), moment.format("%H:%M:%S").to_string()))
}

/// [`utc_parts`] in English: `2026-10-30 14:03:07 UTC`.
pub fn time_words(secs: u64) -> String {
    utc_parts(secs).map_or_else(|| secs.to_string(), |(date, time)| format!("{date} {time} UTC"))
}

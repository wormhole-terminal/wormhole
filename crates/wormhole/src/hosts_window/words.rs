//! The report's facts in the interface's words (the catalog `hosts-window.ftl`). The copied text is
//! core's and stays English.

use gpui_kit::Hsla;
use wormhole_core::ipc::hosts::ProbeFailure;
use wormhole_core::ipc::hosts::StaleReason;
use wormhole_core::ipc::hosts::report::{
    Answer, Elapsed, HostFit, HostRow, SizeUnit, Standing, elapsed, size, utc_parts,
};
use wormhole_term::client::{LinkMode, LinkStats};
use wormhole_text::keys::{
    HOSTS_ANSWER_EXITED, HOSTS_ANSWER_INCOMPATIBLE, HOSTS_ANSWER_NOT_ASKED, HOSTS_FAILURE_HUNG_UP,
    HOSTS_FAILURE_NOT_OURS, HOSTS_FAILURE_OTHER_HOST, HOSTS_FAILURE_TIMED_OUT, HOSTS_FAILURE_UNEXPECTED,
    HOSTS_LINK_DETACHED, HOSTS_LINK_HEADLESS, HOSTS_LINK_LOST, HOSTS_LINK_NONE, HOSTS_LINK_RAW,
    HOSTS_LINK_SYNC, HOSTS_STALE_NO_META, HOSTS_STALE_PID_GONE, HOSTS_STALE_SOCKET_MISSING,
    HOSTS_STALE_UNREADABLE, HOSTS_STANDING_ATTACHED, HOSTS_STANDING_DETACHED, HOSTS_STANDING_DETACHED_NOTE,
    HOSTS_STANDING_ORPHAN, HOSTS_STANDING_ORPHAN_NOTE, HOSTS_STANDING_STALE, hosts_answer_none,
    hosts_answer_running, hosts_duration_days, hosts_duration_hours, hosts_duration_minutes,
    hosts_duration_seconds, hosts_failure_connect, hosts_failure_io, hosts_fit_compatible, hosts_fit_too_new,
    hosts_fit_too_old, hosts_heard_ago, hosts_link_not_responding, hosts_link_traffic, hosts_protocol_minor,
    hosts_quiet_for, hosts_range_not_running, hosts_range_one, hosts_range_span, hosts_size_bytes,
    hosts_size_kib, hosts_size_mib, hosts_standing_attached_to, hosts_started, hosts_time_utc,
};
use wormhole_text::text;

use crate::settings::UiColors;

pub(super) fn standing_label(standing: &Standing) -> &'static str {
    match standing {
        Standing::Attached { .. } => text(HOSTS_STANDING_ATTACHED),
        Standing::Detached => text(HOSTS_STANDING_DETACHED),
        Standing::Orphan => text(HOSTS_STANDING_ORPHAN),
        Standing::Stale(_) => text(HOSTS_STANDING_STALE),
    }
}

pub(super) fn standing_color(ui: UiColors, standing: &Standing) -> Hsla {
    match standing {
        Standing::Attached { .. } => ui.success_text,
        Standing::Detached => ui.info,
        Standing::Orphan => ui.warning_text,
        Standing::Stale(_) => ui.muted,
    }
}

/// The line under a card's badge: where the terminal is, or why the files are left over.
pub(super) fn standing_note(standing: &Standing) -> String {
    match standing {
        Standing::Attached { window, title } => hosts_standing_attached_to(&window.to_string(), title),
        Standing::Detached => text(HOSTS_STANDING_DETACHED_NOTE).to_string(),
        Standing::Orphan => text(HOSTS_STANDING_ORPHAN_NOTE).to_string(),
        Standing::Stale(StaleReason::Unreadable(_)) => text(HOSTS_STALE_UNREADABLE).to_string(),
        Standing::Stale(StaleReason::PidGone) => text(HOSTS_STALE_PID_GONE).to_string(),
        Standing::Stale(StaleReason::SocketMissing) => text(HOSTS_STALE_SOCKET_MISSING).to_string(),
        Standing::Stale(StaleReason::NoMeta) => text(HOSTS_STALE_NO_META).to_string(),
    }
}

pub(super) fn started(row: &HostRow) -> Option<String> {
    let when = time(row.started_at?);
    Some(match row.uptime_secs {
        Some(up) => hosts_started(&when, &elapsed_text(up)),
        None => when,
    })
}

/// A moment in universal time.
fn time(secs: u64) -> String {
    utc_parts(secs).map_or_else(|| secs.to_string(), |(date, time)| hosts_time_utc(&date, &time))
}

/// How long, in the two largest units.
fn elapsed_text(secs: u64) -> String {
    match elapsed(secs) {
        Elapsed::Seconds(s) => hosts_duration_seconds(&s.to_string()),
        Elapsed::MinutesSeconds(m, s) => hosts_duration_minutes(&m.to_string(), &s.to_string()),
        Elapsed::HoursMinutes(h, m) => hosts_duration_hours(&h.to_string(), &m.to_string()),
        Elapsed::DaysHours(d, h) => hosts_duration_days(&d.to_string(), &h.to_string()),
    }
}

/// A size of memory or traffic.
pub(super) fn size_text(bytes: u64) -> String {
    let (unit, number) = size(bytes);
    match unit {
        SizeUnit::Bytes => hosts_size_bytes(&number),
        SizeUnit::Kib => hosts_size_kib(&number),
        SizeUnit::Mib => hosts_size_mib(&number),
    }
}

/// Why a host did not answer.
fn failure(why: &ProbeFailure) -> String {
    match why {
        ProbeFailure::Connect(why) => hosts_failure_connect(why),
        ProbeFailure::NotOurs => text(HOSTS_FAILURE_NOT_OURS).to_string(),
        ProbeFailure::OtherHost => text(HOSTS_FAILURE_OTHER_HOST).to_string(),
        ProbeFailure::HungUp => text(HOSTS_FAILURE_HUNG_UP).to_string(),
        ProbeFailure::Unexpected => text(HOSTS_FAILURE_UNEXPECTED).to_string(),
        ProbeFailure::TimedOut => text(HOSTS_FAILURE_TIMED_OUT).to_string(),
        ProbeFailure::Io(why) => hosts_failure_io(why),
    }
}

/// A host's majors in words: "1", or "1 to 2".
fn range(min: u16, max: u16) -> String {
    if min == max {
        hosts_range_one(&min.to_string())
    } else {
        hosts_range_span(&min.to_string(), &max.to_string())
    }
}

/// This build's protocol, "1 (minor 2)".
pub(super) fn app_protocol(major: u16, minor: u16) -> String {
    hosts_protocol_minor(&major.to_string(), &minor.to_string())
}

/// The host's majors and how they fit this app's; a host that is not running only has its majors.
pub(super) fn protocol(row: &HostRow) -> Option<String> {
    let (min, max) = row.proto?;
    let range = range(min, max);
    Some(match row.fit {
        Some(HostFit::Compatible(_)) => hosts_fit_compatible(&range),
        Some(HostFit::TooOld) => hosts_fit_too_old(&range),
        Some(HostFit::TooNew) => hosts_fit_too_new(&range),
        None => hosts_range_not_running(&range),
    })
}

/// A path for a line: the end of it when it is long, since that is what tells paths apart.
pub(super) fn path(path: &std::path::Path) -> String {
    path_keeping(path, PATH_KEEP)
}

/// [`path`] in at most `keep` characters, for a line that shares its row with others.
pub(super) fn path_keeping(path: &std::path::Path, keep: usize) -> String {
    wormhole_core::view::elide::front(&path.display().to_string(), keep).into_owned()
}

/// How many characters of a path a line shows.
const PATH_KEEP: usize = 72;

pub(super) fn answer(answer: &Answer) -> String {
    match answer {
        Answer::NotAsked => text(HOSTS_ANSWER_NOT_ASKED).to_string(),
        Answer::Yes(info) if info.exited => text(HOSTS_ANSWER_EXITED).to_string(),
        Answer::Yes(info) => hosts_answer_running(
            &info.cols.to_string(),
            &info.rows.to_string(),
            &info.history_lines.to_string(),
        ),
        Answer::Incompatible => text(HOSTS_ANSWER_INCOMPATIBLE).to_string(),
        Answer::No(why) => hosts_answer_none(&failure(why)),
    }
}

/// How long ago, from a duration.
pub(super) fn ago_text(since: std::time::Duration) -> String {
    hosts_heard_ago(&elapsed_text(since.as_secs()))
}

/// How long a link that is not pinged has been quiet.
pub(super) fn quiet_text(since: std::time::Duration) -> String {
    hosts_quiet_for(&elapsed_text(since.as_secs()))
}

pub(super) fn link(link: Option<&LinkStats>) -> String {
    let Some(l) = link else { return text(HOSTS_LINK_NONE).to_string() };
    if let Some(silent) = l.not_responding_for {
        return hosts_link_not_responding(&elapsed_text(silent.as_secs()));
    }
    if l.lost {
        return text(HOSTS_LINK_LOST).to_string();
    }
    if l.detached || l.ended {
        return text(HOSTS_LINK_DETACHED).to_string();
    }
    let mode = match l.mode {
        LinkMode::Sync => text(HOSTS_LINK_SYNC),
        LinkMode::Raw => text(HOSTS_LINK_RAW),
        LinkMode::Headless => text(HOSTS_LINK_HEADLESS),
    };
    hosts_link_traffic(
        mode,
        &size_text(l.bytes_in),
        &size_text(l.bytes_out),
        &l.frames_in.to_string(),
        &l.frames_out.to_string(),
    )
}

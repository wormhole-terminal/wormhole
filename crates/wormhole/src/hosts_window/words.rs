//! The report's facts in the interface's words (the catalog `hosts-window.ftl`). The copied text is
//! core's and stays English.

use gpui_kit::Hsla;
use wormhole_core::ipc::hosts::StaleReason;
use wormhole_core::ipc::hosts::report::{
    Answer, HostFit, HostRow, Standing, bytes_words, duration_words, time_words,
};
use wormhole_term::client::{LinkMode, LinkStats};
use wormhole_text::keys::{
    HOSTS_ANSWER_EXITED, HOSTS_ANSWER_INCOMPATIBLE, HOSTS_ANSWER_NOT_ASKED, HOSTS_LINK_DETACHED,
    HOSTS_LINK_HEADLESS, HOSTS_LINK_LOST, HOSTS_LINK_NONE, HOSTS_LINK_RAW, HOSTS_LINK_SYNC,
    HOSTS_STALE_NO_META, HOSTS_STALE_PID_GONE, HOSTS_STALE_SOCKET_MISSING, HOSTS_STALE_UNREADABLE,
    HOSTS_STANDING_ATTACHED, HOSTS_STANDING_DETACHED, HOSTS_STANDING_DETACHED_NOTE, HOSTS_STANDING_ORPHAN,
    HOSTS_STANDING_ORPHAN_NOTE, HOSTS_STANDING_STALE, hosts_answer_none, hosts_answer_running,
    hosts_fit_compatible, hosts_fit_too_new, hosts_fit_too_old, hosts_link_traffic,
    hosts_standing_attached_to, hosts_started,
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
    let when = time_words(row.started_at?);
    Some(match row.uptime_secs {
        Some(up) => hosts_started(&when, &duration_words(up)),
        None => when,
    })
}

pub(super) fn protocol(row: &HostRow) -> Option<String> {
    let (min, max) = row.proto?;
    let range = format!("{min}..={max}");
    Some(match row.fit? {
        HostFit::Compatible(_) => hosts_fit_compatible(&range),
        HostFit::TooOld => hosts_fit_too_old(&range),
        HostFit::TooNew => hosts_fit_too_new(&range),
    })
}

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
        Answer::No(why) => hosts_answer_none(why),
    }
}

pub(super) fn link(link: Option<&LinkStats>) -> String {
    let Some(l) = link else { return text(HOSTS_LINK_NONE).to_string() };
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
        &bytes_words(l.bytes_in),
        &bytes_words(l.bytes_out),
        &l.frames_in.to_string(),
        &l.frames_out.to_string(),
    )
}

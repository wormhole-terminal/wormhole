//! What the status bar counts and how much of it fits (`wormhole_core::view::layout::status_bar`): which terminal the
//! button is for (the one that has waited longest), the counts after it in the bar's order, and how much the bar says
//! at a window's width. The window hands over each terminal's activity and since when, and draws what comes back.

use std::time::{Duration, Instant};

use wormhole_core::view::layout::status_bar::{Activity, Detail, Entry, Summary};

/// What a terminal is doing, as far as the bar counts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum StatusCountKind {
    Working,
    NeedsYou,
    Done,
    Failed,
    /// Idle or exited: not counted.
    Quiet,
}

impl From<StatusCountKind> for Activity {
    fn from(kind: StatusCountKind) -> Self {
        match kind {
            StatusCountKind::Working => Activity::Working,
            StatusCountKind::NeedsYou => Activity::NeedsYou,
            StatusCountKind::Done => Activity::Done,
            StatusCountKind::Failed => Activity::Failed,
            StatusCountKind::Quiet => Activity::Quiet,
        }
    }
}

impl From<Activity> for StatusCountKind {
    fn from(activity: Activity) -> Self {
        match activity {
            Activity::Working => StatusCountKind::Working,
            Activity::NeedsYou => StatusCountKind::NeedsYou,
            Activity::Done => StatusCountKind::Done,
            Activity::Failed => StatusCountKind::Failed,
            Activity::Quiet => StatusCountKind::Quiet,
        }
    }
}

/// One terminal as the bar sees it: what it does and when that began (the terminal summary's `since`, in milliseconds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct StatusEntryRecord {
    pub kind: StatusCountKind,
    pub since_ms: Option<u64>,
}

/// One count after the button: how many do `kind`, and the first of them (an index into the entries), which a click goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct StatusCountRecord {
    pub kind: StatusCountKind,
    pub count: u32,
    pub terminal: u32,
}

/// What the bar counts.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct StatusCountsRecord {
    /// Who needs input, longest waiting first (indices into the entries): the first is the button's.
    pub needs_you: Vec<u32>,
    /// The counts after the button, in the bar's order.
    pub rest: Vec<StatusCountRecord>,
}

/// How much the bar says at a window's width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum StatusBarDetail {
    Lean,
    Numbers,
    Words,
}

fn index(at: usize) -> u32 {
    u32::try_from(at).unwrap_or(u32::MAX)
}

/// The bar's counts for these terminals (`Summary::of`, `Summary::rest`).
#[uniffi::export]
pub fn status_counts(entries: Vec<StatusEntryRecord>) -> StatusCountsRecord {
    // A terminal with no start counts as the last to have waited; the times are only ever compared with each other.
    let base = Instant::now();
    let names = vec![String::new(); entries.len()];
    let core: Vec<Entry> = entries
        .iter()
        .zip(&names)
        .map(|(entry, name)| Entry {
            activity: entry.kind.into(),
            since: base + Duration::from_millis(entry.since_ms.unwrap_or(u64::MAX >> 24)),
            agent: None,
            name,
        })
        .collect();
    let summary = Summary::of(&core);
    StatusCountsRecord {
        needs_you: summary.needs_you.iter().map(|&at| index(at)).collect(),
        rest: summary
            .rest()
            .iter()
            .map(|counted| StatusCountRecord {
                kind: counted.activity.into(),
                count: index(counted.count),
                terminal: index(counted.terminal),
            })
            .collect(),
    }
}

/// How much the bar says in a window `window_width` points wide.
#[uniffi::export]
pub fn status_detail(window_width: f32) -> StatusBarDetail {
    match Detail::at(window_width) {
        Detail::Lean => StatusBarDetail::Lean,
        Detail::Numbers => StatusBarDetail::Numbers,
        Detail::Words => StatusBarDetail::Words,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: StatusCountKind, since_ms: Option<u64>) -> StatusEntryRecord {
        StatusEntryRecord { kind, since_ms }
    }

    #[test]
    fn the_longest_waiting_is_the_button_and_the_rest_are_counted_in_the_bars_order() {
        let counts = status_counts(vec![
            entry(StatusCountKind::Working, Some(1)),
            entry(StatusCountKind::NeedsYou, Some(900)),
            entry(StatusCountKind::Quiet, None),
            entry(StatusCountKind::NeedsYou, Some(100)),
            entry(StatusCountKind::Failed, Some(5)),
            entry(StatusCountKind::Working, Some(2)),
        ]);
        assert_eq!(counts.needs_you, [3, 1], "waiting since 100 before 900");
        assert_eq!(
            counts.rest,
            [
                StatusCountRecord { kind: StatusCountKind::NeedsYou, count: 1, terminal: 1 },
                StatusCountRecord { kind: StatusCountKind::Working, count: 2, terminal: 0 },
                StatusCountRecord { kind: StatusCountKind::Failed, count: 1, terminal: 4 },
            ]
        );
    }

    #[test]
    fn the_words_go_first_then_the_size() {
        assert_eq!(status_detail(1400.0), StatusBarDetail::Words);
        assert_eq!(status_detail(10.0), StatusBarDetail::Lean);
    }
}

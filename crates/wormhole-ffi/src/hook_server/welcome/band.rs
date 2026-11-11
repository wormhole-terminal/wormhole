//! The example band the sidebar step puts in the window's sidebar: core's rows (`welcome::demo`), which Swift
//! draws with the sidebar's own row. Nothing about them is a terminal.

use std::time::Duration;

use wormhole_core::onboarding::welcome::demo;
use wormhole_core::view::sidebar::RowState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WelcomeDemoState {
    Working,
    NeedsYou,
    Finished,
}

/// One example row.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct WelcomeDemoRow {
    pub name: String,
    pub state: WelcomeDemoState,
    /// The second line, in the sidebar's words.
    pub detail: String,
    /// What VoiceOver reads: it says it is an example.
    pub spoken: String,
}

/// The band's title ("Example").
#[uniffi::export]
pub fn welcome_demo_title() -> String {
    demo::BAND.to_string()
}

/// The band as VoiceOver reads it.
#[uniffi::export]
pub fn welcome_demo_label() -> String {
    demo::band_label()
}

/// The three rows `elapsed_ms` after the band went up; `still` (Reduce Motion) holds them as at the start.
#[uniffi::export]
pub fn welcome_demo_rows(elapsed_ms: u64, still: bool) -> Vec<WelcomeDemoRow> {
    demo::rows(Duration::from_millis(elapsed_ms), still)
        .into_iter()
        .map(|row| WelcomeDemoRow {
            name: row.name.to_string(),
            state: match row.state {
                RowState::NeedsYou => WelcomeDemoState::NeedsYou,
                RowState::Finished => WelcomeDemoState::Finished,
                _ => WelcomeDemoState::Working,
            },
            detail: row.detail,
            spoken: row.spoken,
        })
        .collect()
}

/// How long until the rows next change, for the timer that redraws them; `None` under Reduce Motion.
#[uniffi::export]
pub fn welcome_demo_next_ms(elapsed_ms: u64, still: bool) -> Option<u64> {
    demo::until_change(Duration::from_millis(elapsed_ms), still).map(|wait| wait.as_millis() as u64)
}

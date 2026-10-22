//! The example band the sidebar step puts in the window's sidebar, as data: three rows whose
//! statuses cycle working, needs you, done, so the step's text can point at them ("←") and say what
//! each means. The app draws them with the sidebar's own row and band components; nothing about
//! them is a terminal (they are never listed, saved, numbered or reachable by a key), so what to
//! show and what to say lives here and the tests pin it.
//!
//! The rows are staggered: at any moment one is working, one needs you and one is done, and each
//! moves to the next state every [`STEP`], so the three states are always side by side and the cycle
//! is visible. Under Reduce Motion nothing cycles: the same three rows, held at the start.

use std::time::Duration;

use crate::sidebar::RowState;
use crate::sidebar::spoken::state_word;
use crate::sidebar::text::agent_detail;

/// The band's title.
pub const BAND: &str = "Example";

/// How long a row stays in one state before it moves to the next.
pub const STEP: Duration = Duration::from_millis(2000);

/// The states in the order a row goes through them.
const CYCLE: [RowState; 3] = [RowState::Working, RowState::NeedsYou, RowState::Finished];

/// How long ago the finished row says it finished: fixed, so the text holds still between changes.
const FINISHED_AGO: Duration = Duration::from_secs(2 * 60);

/// One example terminal: what it is called, what its agent says while it works and when it asks.
struct Example {
    name: &'static str,
    working: &'static str,
    question: &'static str,
}

const EXAMPLES: [Example; 3] = [
    Example { name: "Fix the login bug", working: "Editing src/login.rs…", question: "Run the tests?" },
    Example { name: "Add tests", working: "Reading src/orders.rs…", question: "Create tests/orders.rs?" },
    Example {
        name: "Update the docs", working: "Editing README.md…", question: "Overwrite docs/setup.md?"
    },
];

/// One row as the sidebar shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub name: &'static str,
    pub state: RowState,
    /// The second line: what the agent is doing, asking, or how long ago it finished.
    pub detail: String,
    /// What VoiceOver reads: it says it is an example, since it is not one of the person's terminals.
    pub spoken: String,
}

/// The band as VoiceOver reads it.
pub fn band_label() -> String {
    format!("{BAND}, three example terminals showing what working, needs you and finished look like")
}

/// The three rows `elapsed` after the band went up. With `still` (Reduce Motion) they are the rows as
/// they were at the start, however long it has been.
pub fn rows(elapsed: Duration, still: bool) -> [Row; 3] {
    let phase = if still { 0 } else { (elapsed.as_millis() / STEP.as_millis()) as usize };
    std::array::from_fn(|ix| {
        let example = &EXAMPLES[ix];
        let state = CYCLE[(phase + ix) % CYCLE.len()];
        let message = match state {
            RowState::Working => Some(example.working),
            RowState::NeedsYou => Some(example.question),
            _ => None,
        };
        let detail = agent_detail(state, message, FINISHED_AGO).unwrap_or_default();
        let spoken = format!("{}, {}, example", example.name, state_word(state));
        Row { name: example.name, state, detail, spoken }
    })
}

/// How long until the rows next change, for the timer that redraws them. `None` under Reduce Motion:
/// they never do.
pub fn until_change(elapsed: Duration, still: bool) -> Option<Duration> {
    if still {
        return None;
    }
    let step = STEP.as_millis();
    let into = elapsed.as_millis() % step;
    Some(Duration::from_millis((step - into) as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn states(elapsed: Duration, still: bool) -> Vec<RowState> {
        rows(elapsed, still).iter().map(|row| row.state).collect()
    }

    #[test]
    fn the_three_states_are_side_by_side_and_each_row_cycles_through_them() {
        use RowState::*;
        assert_eq!(states(Duration::ZERO, false), [Working, NeedsYou, Finished]);
        assert_eq!(states(STEP, false), [NeedsYou, Finished, Working]);
        assert_eq!(states(STEP * 2, false), [Finished, Working, NeedsYou]);
        assert_eq!(states(STEP * 3, false), states(Duration::ZERO, false), "and round again");
        // Working, then needs you, then done: the order the step's text names them in.
        let first: Vec<RowState> = (0..3).map(|n| states(STEP * n, false)[0]).collect();
        assert_eq!(first, [Working, NeedsYou, Finished]);
        // Between two changes nothing moves.
        assert_eq!(states(STEP - Duration::from_millis(1), false), states(Duration::ZERO, false));
    }

    #[test]
    fn reduce_motion_holds_the_rows_still() {
        for elapsed in [Duration::ZERO, STEP, STEP * 5 + Duration::from_millis(7)] {
            assert_eq!(rows(elapsed, true), rows(Duration::ZERO, true));
        }
        assert_eq!(until_change(STEP, true), None, "no timer needed");
    }

    #[test]
    fn the_redraw_timer_wakes_at_the_next_change() {
        assert_eq!(until_change(Duration::ZERO, false), Some(STEP));
        assert_eq!(until_change(Duration::from_millis(500), false), Some(STEP - Duration::from_millis(500)));
        assert_eq!(until_change(STEP, false), Some(STEP), "exactly on a change: a whole step to the next");
    }

    #[test]
    fn each_row_says_what_its_agent_does_in_the_sidebars_words() {
        let at_start = rows(Duration::ZERO, false);
        assert_eq!(at_start[0].detail, "Editing src/login.rs…");
        assert_eq!(at_start[1].detail, "Needs you · Create tests/orders.rs?");
        assert_eq!(at_start[2].detail, "Finished · 2m ago");
    }

    #[test]
    fn voiceover_is_told_they_are_examples() {
        let at_start = rows(Duration::ZERO, false);
        assert_eq!(at_start[0].spoken, "Fix the login bug, working, example");
        assert_eq!(at_start[1].spoken, "Add tests, needs you, example");
        assert_eq!(at_start[2].spoken, "Update the docs, finished, example");
        assert!(band_label().starts_with("Example,"));
    }
}

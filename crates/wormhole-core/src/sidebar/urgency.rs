//! Which terminal to show when only one can be: a folded group's head, a tab switcher's card, the
//! status bar's button. They all pick by the same order of urgency (the boards' RF-12), so this is
//! the one place that holds it. The app maps what a terminal's tile says into [`Urgency`]; here
//! only the order and the picking live, so they are tested without GPUI.

use super::RowState;

/// What a terminal has to say, loudest first: the derived order is the order of urgency, so
/// `NeedsYou < Failed < ... < Idle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Urgency {
    /// An agent asks something.
    NeedsYou,
    /// A command or an agent ended on an error nobody has looked at.
    Failed,
    /// An agent works.
    Working,
    /// A terminal without an agent rang its bell.
    Bell,
    /// A terminal has output nobody has seen.
    NewOutput,
    /// A command runs in a terminal without an agent.
    Running,
    /// An agent finished and nobody has looked yet.
    Done,
    /// Nothing happens: an agent waiting for a prompt, a shell at its prompt, an ended shell.
    Idle,
}

impl Urgency {
    /// Every urgency, loudest first.
    pub const ORDER: [Urgency; 8] = [
        Urgency::NeedsYou,
        Urgency::Failed,
        Urgency::Working,
        Urgency::Bell,
        Urgency::NewOutput,
        Urgency::Running,
        Urgency::Done,
        Urgency::Idle,
    ];

    /// 0 for the loudest, counting up.
    pub fn rank(self) -> u8 {
        self as u8
    }

    /// Needs the person now: what a folded group's alert dot and the sidebar's attention count
    /// stand for.
    pub fn is_alert(self) -> bool {
        matches!(self, Urgency::NeedsYou | Urgency::Failed)
    }
}

/// The urgency of a row's state, for callers that know nothing finer (no bell, no unread output,
/// no command in the foreground).
impl From<RowState> for Urgency {
    fn from(state: RowState) -> Self {
        match state {
            RowState::NeedsYou => Urgency::NeedsYou,
            RowState::Failed => Urgency::Failed,
            RowState::Working => Urgency::Working,
            RowState::Finished => Urgency::Done,
            RowState::Idle | RowState::Exited => Urgency::Idle,
        }
    }
}

/// The index of the loudest of `urgencies`; the first of equally loud ones, so a list keeps its own
/// order among ties. `None` for an empty list.
pub fn loudest_of(urgencies: impl IntoIterator<Item = Urgency>) -> Option<usize> {
    urgencies.into_iter().enumerate().min_by_key(|&(_, urgency)| urgency).map(|(at, _)| at)
}

/// The loudest of `items` by `urgency`, the first among equals.
pub fn loudest<T>(items: impl IntoIterator<Item = T>, urgency: impl Fn(&T) -> Urgency) -> Option<T> {
    items.into_iter().min_by_key(|item| urgency(item))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_order_is_needs_you_failed_working_bell_new_output_running_done_idle() {
        let mut sorted = Urgency::ORDER;
        sorted.sort();
        assert_eq!(sorted, Urgency::ORDER);
        let ranks: Vec<u8> = Urgency::ORDER.iter().map(|u| u.rank()).collect();
        assert_eq!(ranks, (0..8).collect::<Vec<u8>>());
        assert!(Urgency::NeedsYou < Urgency::Failed && Urgency::Failed < Urgency::Working);
        assert!(Urgency::Working < Urgency::Bell && Urgency::Bell < Urgency::NewOutput);
        assert!(Urgency::NewOutput < Urgency::Running && Urgency::Running < Urgency::Done);
        assert!(Urgency::Done < Urgency::Idle);
    }

    #[test]
    fn the_loudest_wins_and_ties_keep_the_first() {
        use Urgency::*;
        assert_eq!(loudest_of([Idle, Done, Working, Failed, Working]), Some(3));
        assert_eq!(loudest_of([Working, Bell, Working]), Some(0));
        assert_eq!(loudest_of([Idle, Idle]), Some(0));
        assert_eq!(loudest_of([]), None);
        assert_eq!(loudest(["a", "b", "c"], |&n| if n == "b" { NeedsYou } else { Running }), Some("b"));
        assert_eq!(loudest(Vec::<u8>::new(), |_| Idle), None);
    }

    #[test]
    fn working_beats_the_bell_and_unread_output_beats_a_running_command() {
        use Urgency::*;
        assert_eq!(loudest_of([Bell, Working]), Some(1));
        assert_eq!(loudest_of([Running, NewOutput]), Some(1));
        assert_eq!(loudest_of([Done, Running]), Some(1));
    }

    #[test]
    fn only_needs_you_and_failed_are_alerts() {
        let alerts: Vec<Urgency> = Urgency::ORDER.into_iter().filter(|u| u.is_alert()).collect();
        assert_eq!(alerts, [Urgency::NeedsYou, Urgency::Failed]);
    }

    #[test]
    fn a_rows_state_maps_to_its_urgency() {
        assert_eq!(Urgency::from(RowState::NeedsYou), Urgency::NeedsYou);
        assert_eq!(Urgency::from(RowState::Finished), Urgency::Done);
        assert_eq!(Urgency::from(RowState::Exited), Urgency::Idle);
        assert_eq!(Urgency::from(RowState::Idle), Urgency::Idle);
    }
}

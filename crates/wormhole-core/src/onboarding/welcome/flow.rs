//! The terminal welcome's steps: which one is up, and where Back, Continue and Skip lead. It is its
//! own sequence, not core's `onboarding` flow, which is the intro window's (where you work, how it
//! looks, the coding agents) and stays as it is while the window is what the first run shows.
//!
//! 1. Welcome: the mark, the name and the sidebar's three lines;
//! 2. Look: theme, colour, font, ligatures;
//! 3. Shown: how much the window shows, a walk of questions with a preview for each answer. The walk
//!    is inside the step (`tailoring`): its questions are not steps of the flow, so Back on the first
//!    question is the only Back that leaves it;
//! 4. Notify: what reaches you when an agent needs input or finishes;
//! 5. Sidebar: the example band in the real sidebar, and the agents found with their hooks (or, with
//!    none, a primer);
//! 6. Extras: what else is here, read-only.
//!
//! A replay of the welcome (Help → Welcome to Wormhole) is the first step alone. Moving through the
//! steps never asks the app for anything beyond the look it has been shown: the flow has no event for
//! an install, a notification choice or an answer of the walk, those are presses on a row (`model`).

/// A step of the terminal welcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Step {
    Welcome,
    Look,
    Shown,
    Notify,
    Sidebar,
    Extras,
}

impl Step {
    fn next(self) -> Option<Step> {
        Some(match self {
            Step::Welcome => Step::Look,
            Step::Look => Step::Shown,
            Step::Shown => Step::Notify,
            Step::Notify => Step::Sidebar,
            Step::Sidebar => Step::Extras,
            Step::Extras => return None,
        })
    }

    fn previous(self) -> Step {
        match self {
            Step::Welcome | Step::Look => Step::Welcome,
            Step::Shown => Step::Look,
            Step::Notify => Step::Shown,
            Step::Sidebar => Step::Notify,
            Step::Extras => Step::Sidebar,
        }
    }
}

/// What the person did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Event {
    Continue,
    Back,
    /// Esc: close without going further.
    Skip,
}

/// What the program is to do about an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    /// Nothing beyond showing the step now current.
    None,
    /// The welcome is over; `skipped` when Esc ended it.
    End { skipped: bool },
}

/// The flow's state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Flow {
    step: Step,
    /// A replay: the first step is all there is, and it is the last.
    replay: bool,
}

impl Flow {
    /// The first run, from the first step.
    pub(super) fn new() -> Self {
        Self { step: Step::Welcome, replay: false }
    }

    /// The welcome shown again once the first run is over: its first step alone.
    pub(super) fn replay() -> Self {
        Self { step: Step::Welcome, replay: true }
    }

    pub(super) fn step(&self) -> Step {
        self.step
    }

    pub(super) fn is_replay(&self) -> bool {
        self.replay
    }

    pub(super) fn apply(&mut self, event: Event) -> Action {
        match event {
            Event::Skip => Action::End { skipped: true },
            Event::Back => {
                self.step = self.step.previous();
                Action::None
            }
            Event::Continue => match self.step.next() {
                Some(next) if !self.replay => {
                    self.step = next;
                    Action::None
                }
                _ => Action::End { skipped: false },
            },
        }
    }
}

impl Default for Flow {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continue_walks_the_six_steps_and_the_last_one_ends() {
        let mut flow = Flow::new();
        let mut seen = vec![flow.step()];
        while flow.apply(Event::Continue) == Action::None {
            seen.push(flow.step());
        }
        assert_eq!(seen, [Step::Welcome, Step::Look, Step::Shown, Step::Notify, Step::Sidebar, Step::Extras]);
        assert_eq!(flow.step(), Step::Extras, "it ends where the last step is");
    }

    #[test]
    fn back_walks_them_the_other_way_and_stops_at_the_first() {
        let mut flow = Flow::new();
        for _ in 0..5 {
            flow.apply(Event::Continue);
        }
        let mut seen = vec![flow.step()];
        for _ in 0..6 {
            assert_eq!(flow.apply(Event::Back), Action::None);
            seen.push(flow.step());
        }
        assert_eq!(
            seen,
            [
                Step::Extras,
                Step::Sidebar,
                Step::Notify,
                Step::Shown,
                Step::Look,
                Step::Welcome,
                Step::Welcome
            ]
        );
    }

    #[test]
    fn skip_ends_from_any_step_as_skipped() {
        for steps in 0..5 {
            let mut flow = Flow::new();
            for _ in 0..steps {
                flow.apply(Event::Continue);
            }
            assert_eq!(flow.apply(Event::Skip), Action::End { skipped: true });
        }
    }

    #[test]
    fn a_replay_is_the_first_step_alone() {
        let mut flow = Flow::replay();
        assert!(flow.is_replay());
        assert_eq!(flow.apply(Event::Continue), Action::End { skipped: false });
        assert_eq!(flow.step(), Step::Welcome, "it never goes on to the setup");
        assert_eq!(flow.apply(Event::Back), Action::None);
        assert_eq!(flow.step(), Step::Welcome);
        // The first run's first step is not the end.
        assert_eq!(Flow::new().apply(Event::Continue), Action::None);
    }
}

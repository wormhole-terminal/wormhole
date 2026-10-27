//! The Shown step's state: the walk through how much the window shows (core's `onboarding::tailor`,
//! which the intro window's Look step walks too), where the cursor is on the page it draws, and what the
//! app has taken of it.
//!
//! The walk decides which question is asked and what each answer makes of the window; this keeps it and
//! says, for each thing the person does, what the app is to be told: the walk's state before the answer
//! and after it, and only when the answer changed it. Moving between questions, or pressing the answer
//! already in use, asks for nothing.
//!
//! The walk shows its answers at once, before the app has said it took them. It keeps the last state the
//! app acknowledged, so that a request the app refused is taken back (the page would otherwise show a
//! window that is not there), and where the right panel really ended up, which a window too narrow for
//! it decides.

use std::collections::VecDeque;

use crate::config::Settings;
use crate::config::feature_table::FeatureId;
use crate::config::presets::{Shown, offered_tools};
use crate::onboarding::tailor::{Step as Asked, Walk};

/// The window as it was shown before an answer, and as it is with it: what the app is told.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Change {
    pub from: Shown,
    pub shown: Shown,
}

/// What a press on the page does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Press {
    /// Nothing the model need act on: the walk moved, or there was nothing to do.
    Stay,
    /// Back from the first question: the step before.
    Leave,
    /// The window now shows something else.
    Change(Change),
}

/// A walk and the cursor on its page.
#[derive(Clone, Debug)]
pub(super) struct Tailoring {
    walk: Walk,
    /// The cursor among the answers; `None` is the page's button, where it starts, so that Return
    /// goes on and keeps things as they are.
    at: Option<usize>,
    /// The optional tools this build has, for the sketches' tabs (read once, from the same defaults
    /// the app starts from).
    offered: Vec<FeatureId>,
    /// What the window showed when the app last took a request (at first, when the walk began).
    acked: Shown,
    /// What the walk showed with each request sent and not yet answered, oldest first.
    pending: VecDeque<Shown>,
}

impl Tailoring {
    /// A walk from what the window shows now.
    pub(super) fn new(shown: Shown) -> Self {
        Self {
            acked: shown.clone(),
            walk: Walk::new(shown),
            at: None,
            offered: offered_tools(&Settings::default()),
            pending: VecDeque::new(),
        }
    }

    pub(super) fn walk(&self) -> &Walk {
        &self.walk
    }

    /// The question asked and its place, or `None` on the summary.
    pub(super) fn asking(&self) -> Option<Asked> {
        self.walk.step()
    }

    pub(super) fn offered(&self) -> &[FeatureId] {
        &self.offered
    }

    /// How many answers the page lists: the question's, none on the summary.
    pub(super) fn answers(&self) -> usize {
        self.asking().map_or(0, |step| step.question.answers().len())
    }

    /// Where the cursor is among the page's lines: the answers, then the button.
    pub(super) fn cursor(&self) -> usize {
        self.at.map_or(self.answers(), |at| at.min(self.answers()))
    }

    /// Put the cursor on line `line`; the one past the answers is the button.
    pub(super) fn set_cursor(&mut self, line: usize) {
        self.at = (line < self.answers()).then_some(line);
    }

    /// The answer the page shows the detail of: the one under the cursor, else the one in use.
    pub(super) fn highlighted(&self) -> Option<usize> {
        let step = self.asking()?;
        self.at.or_else(|| step.question.picked(&self.walk.shown))
    }

    // ---------------------------------------------------------------------------------- presses

    /// What a character does on the page: a digit gives that answer, `b` goes back and `r` starts over.
    pub(super) fn key(&mut self, c: char) -> Press {
        match typed(c) {
            Some(Typed::Answer(ix)) => self.answer(ix),
            Some(Typed::Back) => self.back(),
            Some(Typed::StartOver) => self.start_over(),
            None => Press::Stay,
        }
    }

    /// Give answer `ix` to the question asked and go on to the next.
    pub(super) fn answer(&mut self, ix: usize) -> Press {
        let from = self.walk.shown.clone();
        if self.walk.answer(ix).is_none() {
            return Press::Stay;
        }
        self.at = None;
        self.changed(from)
    }

    /// Back to the question before (from the summary, the last one); from the first, out of the step.
    /// The answers stay.
    pub(super) fn back(&mut self) -> Press {
        self.at = None;
        if self.walk.back() { Press::Stay } else { Press::Leave }
    }

    /// From the first question again, with the window as it was before the walk.
    pub(super) fn start_over(&mut self) -> Press {
        let from = self.walk.shown.clone();
        self.at = None;
        self.walk.start_over();
        self.changed(from)
    }

    fn changed(&self, from: Shown) -> Press {
        if self.walk.shown == from {
            return Press::Stay;
        }
        Press::Change(Change { from, shown: self.walk.shown.clone() })
    }

    // ------------------------------------------------------------------------------ the app's word

    /// A request showing `shown` went to the app.
    pub(super) fn sent(&mut self, shown: &Shown) {
        self.pending.push_back(shown.clone());
    }

    /// The app took the oldest request. `panel_open` is whether the panel is on screen now, which a
    /// window too narrow for it keeps closed whatever was asked: the walk shows what is true.
    pub(super) fn acknowledged(&mut self, panel_open: Option<bool>) {
        let Some(mut shown) = self.pending.pop_front() else { return };
        if let Some(open) = panel_open.filter(|open| *open != shown.panel_open) {
            shown.panel_open = open;
            self.walk.shown.panel_open = open;
        }
        self.acked = shown;
    }

    /// The app refused a request: the walk shows the window as the app last took it, and the requests
    /// still on their way, made from a state that never was, are let go of.
    pub(super) fn refused(&mut self) {
        self.pending.clear();
        self.walk.shown = self.acked.clone();
    }
}

/// What a character does on the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Typed {
    /// A digit: answer number `n` (from 0) of the question asked.
    Answer(usize),
    Back,
    StartOver,
}

/// What `c` does on the Shown step, if anything: a digit gives that answer, `b` goes back a question
/// and `r` starts the walk over.
pub(super) fn typed(c: char) -> Option<Typed> {
    match c {
        'b' => Some(Typed::Back),
        'r' => Some(Typed::StartOver),
        _ => c.to_digit(10).and_then(|n| usize::try_from(n).ok()?.checked_sub(1)).map(Typed::Answer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::presets::Preset;
    use crate::onboarding::tailor::Question;

    fn balanced() -> Tailoring {
        Tailoring::new(Shown::of(Preset::Balanced))
    }

    fn change(from: Preset, to: Preset) -> Press {
        Press::Change(Change { from: Shown::of(from), shown: Shown::of(to) })
    }

    #[test]
    fn the_cursor_starts_on_the_button_and_an_answer_puts_it_back_there() {
        let mut t = balanced();
        assert_eq!(t.cursor(), 3, "past the three answers of the first question");
        t.set_cursor(1);
        assert_eq!((t.cursor(), t.highlighted()), (1, Some(1)));
        assert_eq!(t.answer(0), change(Preset::Balanced, Preset::Zen));
        assert_eq!(t.cursor(), 3, "the next question's button");
        t.set_cursor(9);
        assert_eq!(t.cursor(), t.answers(), "a line past the button is the button");
    }

    #[test]
    fn only_an_answer_that_changes_the_window_tells_the_app_and_it_says_from_what() {
        let mut t = balanced();
        assert_eq!(t.answer(1), Press::Stay, "Balanced is what it already was");
        assert_eq!(t.asking().map(|step| step.question), Some(Question::TitleRow), "but it went on");
        assert_eq!(t.back(), Press::Stay);
        assert_eq!(t.answer(0), change(Preset::Balanced, Preset::Zen));
        assert_eq!(t.start_over(), change(Preset::Zen, Preset::Balanced));
        assert_eq!(t.start_over(), Press::Stay, "nothing to put back");
        assert_eq!(t.answer(9), Press::Stay);
    }

    #[test]
    fn back_leaves_the_step_from_the_first_question_only() {
        let mut t = balanced();
        assert_eq!(t.back(), Press::Leave);
        t.answer(1);
        assert_eq!(t.key('b'), Press::Stay);
        assert_eq!(t.key('b'), Press::Leave);
    }

    #[test]
    fn the_summary_has_no_answers_and_back_leaves_it_for_the_last_question() {
        let mut t = balanced();
        while t.asking().is_some() {
            t.answer(0);
        }
        assert_eq!((t.answers(), t.cursor(), t.highlighted()), (0, 0, None));
        assert_eq!(t.back(), Press::Stay);
        assert_eq!(t.asking().map(|step| step.question), Some(Question::Tools));
    }

    #[test]
    fn digits_answer_and_b_and_r_go_back_and_start_over() {
        assert_eq!(typed('1'), Some(Typed::Answer(0)));
        assert_eq!(typed('3'), Some(Typed::Answer(2)));
        assert_eq!(typed('0'), None, "there is no answer before the first");
        assert_eq!(
            (typed('b'), typed('r'), typed('x'), typed('٣')),
            (Some(Typed::Back), Some(Typed::StartOver), None, None)
        );
    }

    /// The walk is optimistic: a refused request takes it back to what the app last took.
    #[test]
    fn a_refused_request_takes_the_walk_back_to_what_the_app_last_took() {
        let mut t = balanced();
        let Press::Change(zen) = t.answer(0) else { panic!("a change") };
        t.sent(&zen.shown);
        t.acknowledged(None);
        let Press::Change(path) = t.answer(1) else { panic!("a change") };
        t.sent(&path.shown);
        let Press::Change(status) = t.answer(1) else { panic!("a change") };
        t.sent(&status.shown);
        t.refused();
        assert_eq!(t.walk().shown, Shown::of(Preset::Zen), "as Zen was taken and the rest was not");
        // A later answer is made from there, and a stray acknowledgement of the lost ones is ignored.
        t.acknowledged(None);
        assert_eq!(t.back(), Press::Stay);
        let with_status = Shown { status_bar: true, ..Shown::of(Preset::Zen) };
        assert_eq!(t.answer(1), Press::Change(Change { from: Shown::of(Preset::Zen), shown: with_status }));
    }

    #[test]
    fn a_window_too_narrow_for_the_panel_says_so_and_the_walk_follows() {
        let mut t = balanced();
        let Press::Change(everything) = t.answer(2) else { panic!("a change") };
        t.sent(&everything.shown);
        assert!(t.walk().shown.panel_open);
        t.acknowledged(Some(false));
        assert!(!t.walk().shown.panel_open, "it never opened");
        t.refused();
        assert!(!t.walk().shown.panel_open, "and a refusal later puts back what is true");
    }
}

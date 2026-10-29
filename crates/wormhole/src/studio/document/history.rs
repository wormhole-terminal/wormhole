//! Undo and redo as snapshots of the file: a few KB each, and obviously right where a patch
//! would have to be inverted. A drag of a colour is one step however many values it passes
//! through (a gesture). Saving does not clear it; loading another theme does. Pure.

use wormhole_core::theme::tokens::ThemeFile;

/// The most steps kept; the oldest go first.
const MAX_STEPS: usize = 200;

/// What a step changed, for the Edit menu's "Undo Change “primary”".
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::studio) enum StepLabel {
    /// A token got a value.
    Token(String),
    /// A token went back to what the rules work out.
    Reset(String),
    Rename,
    /// Revert to Saved.
    Revert,
}

/// The file as it was before a change, and what was selected.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::studio) struct Step {
    pub(in crate::studio) label: StepLabel,
    pub(in crate::studio) file: ThemeFile,
    pub(in crate::studio) selected: Option<String>,
}

#[derive(Default)]
pub(in crate::studio) struct UndoStack {
    undo: Vec<Step>,
    redo: Vec<Step>,
    /// The label of the open gesture: further changes with it are part of the same step.
    gesture: Option<StepLabel>,
}

impl UndoStack {
    /// A change is about to be made: keep `before`. With `gesture`, changes that follow with
    /// the same label (a drag) are not kept again until [`Self::end_gesture`].
    pub(in crate::studio) fn record(&mut self, before: Step, gesture: bool) {
        if gesture && self.gesture.as_ref() == Some(&before.label) {
            return;
        }
        self.gesture = gesture.then(|| before.label.clone());
        self.redo.clear();
        self.undo.push(before);
        if self.undo.len() > MAX_STEPS {
            self.undo.remove(0);
        }
    }

    /// The drag, or the run of key presses, is over: the next change is a step of its own.
    pub(in crate::studio) fn end_gesture(&mut self) {
        self.gesture = None;
    }

    /// Step back: the file before the last change. `now` is the file as it stands, kept for redo.
    pub(in crate::studio) fn undo(&mut self, now: &ThemeFile, selected: Option<String>) -> Option<Step> {
        self.gesture = None;
        let step = self.undo.pop()?;
        self.redo.push(Step { label: step.label.clone(), file: now.clone(), selected });
        Some(step)
    }

    /// Step forward again.
    pub(in crate::studio) fn redo(&mut self, now: &ThemeFile, selected: Option<String>) -> Option<Step> {
        self.gesture = None;
        let step = self.redo.pop()?;
        self.undo.push(Step { label: step.label.clone(), file: now.clone(), selected });
        Some(step)
    }

    pub(in crate::studio) fn undo_label(&self) -> Option<&StepLabel> {
        self.undo.last().map(|s| &s.label)
    }

    pub(in crate::studio) fn redo_label(&self) -> Option<&StepLabel> {
        self.redo.last().map(|s| &s.label)
    }

    pub(in crate::studio) fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub(in crate::studio) fn redo_len(&self) -> usize {
        self.redo.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(primary: &str) -> ThemeFile {
        let mut f = ThemeFile::default();
        f.ui.insert("primary".into(), primary.into());
        f
    }

    fn step(primary: &str) -> Step {
        Step { label: StepLabel::Token("primary".into()), file: file(primary), selected: None }
    }

    #[test]
    fn undo_and_redo_walk_the_snapshots() {
        let mut stack = UndoStack::default();
        stack.record(step("#111"), false);
        stack.record(step("#222"), false);
        let back = stack.undo(&file("#333"), None).expect("a step");
        assert_eq!(back.file, file("#222"));
        let back = stack.undo(&file("#222"), None).expect("a step");
        assert_eq!(back.file, file("#111"));
        assert!(stack.undo(&file("#111"), None).is_none());
        let forward = stack.redo(&file("#111"), None).expect("a step");
        assert_eq!(forward.file, file("#222"));
        assert_eq!((stack.undo_len(), stack.redo_len()), (1, 1));
    }

    #[test]
    fn a_new_change_clears_redo() {
        let mut stack = UndoStack::default();
        stack.record(step("#111"), false);
        stack.undo(&file("#222"), None);
        assert_eq!(stack.redo_len(), 1);
        stack.record(step("#111"), false);
        assert_eq!(stack.redo_len(), 0);
    }

    #[test]
    fn a_drag_is_one_step_until_it_ends() {
        let mut stack = UndoStack::default();
        for primary in ["#111", "#222", "#333"] {
            stack.record(step(primary), true);
        }
        assert_eq!(stack.undo_len(), 1, "only the file before the drag");
        stack.end_gesture();
        stack.record(step("#444"), true);
        assert_eq!(stack.undo_len(), 2);
        let back = stack.undo(&file("#555"), None).expect("a step");
        assert_eq!(back.file, file("#444"));
        // A single change in between closes a gesture too.
        let mut stack = UndoStack::default();
        stack.record(step("#111"), true);
        stack.record(step("#222"), false);
        stack.record(step("#333"), true);
        assert_eq!(stack.undo_len(), 3);
    }

    #[test]
    fn the_oldest_steps_go_past_the_cap() {
        let mut stack = UndoStack::default();
        for n in 0..MAX_STEPS + 5 {
            stack.record(step(&format!("#{n}")), false);
        }
        assert_eq!(stack.undo_len(), MAX_STEPS);
    }

    #[test]
    fn labels_follow_the_top_of_each_stack() {
        let mut stack = UndoStack::default();
        assert_eq!(stack.undo_label(), None);
        stack.record(Step { label: StepLabel::Rename, file: file("#1"), selected: None }, false);
        assert_eq!(stack.undo_label(), Some(&StepLabel::Rename));
        stack.undo(&file("#2"), None);
        assert_eq!((stack.undo_label(), stack.redo_label()), (None, Some(&StepLabel::Rename)));
    }
}

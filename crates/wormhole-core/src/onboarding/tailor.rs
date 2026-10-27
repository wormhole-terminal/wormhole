//! The first run's walk through how much the window shows, one question at a time, as a shell
//! prompt's configuration wizard asks it: each question offers two or three answers (numbered, each
//! drawn as a preview where it is shown), Back goes to the question before, Start over to the first,
//! and after the last a summary. The first question picks a preset ([`Preset`]), which answers all the
//! others at once; the ones after it change one thing each, and show the answer the preset gave.
//!
//! The window's setup (the Look step) and `Wormhole welcome` both walk this; it decides nothing about
//! drawing. Every answer is applied as it is given, so the window behind shows it.

use wormhole_text::Key;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

use crate::config::presets::{Preset, Shown, TitleShown, ToolsShown};

/// One question of the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Question {
    Start,
    TitleRow,
    StatusBar,
    Minimap,
    Dividers,
    Panel,
    Tools,
}

/// An answer a question offers: what it is called and a line about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Answer {
    pub label: Key,
    pub detail: Key,
}

const fn answer(label: Key, detail: Key) -> Answer {
    Answer { label, detail }
}

const START: &[Answer] = &[
    answer(TAILOR_ZEN, TAILOR_ZEN_DETAIL),
    answer(TAILOR_BALANCED, TAILOR_BALANCED_DETAIL),
    answer(TAILOR_EVERYTHING, TAILOR_EVERYTHING_DETAIL),
];
const TITLE_ROW: &[Answer] = &[
    answer(TAILOR_TITLE_NAME, TAILOR_TITLE_NAME_DETAIL),
    answer(TAILOR_TITLE_PATH, TAILOR_TITLE_PATH_DETAIL),
    answer(TAILOR_TITLE_EVERYTHING, TAILOR_TITLE_EVERYTHING_DETAIL),
];
const STATUS_BAR: &[Answer] =
    &[answer(TAILOR_OFF, TAILOR_STATUS_BAR_OFF), answer(TAILOR_ON, TAILOR_STATUS_BAR_ON)];
const MINIMAP: &[Answer] = &[answer(TAILOR_OFF, TAILOR_MINIMAP_OFF), answer(TAILOR_ON, TAILOR_MINIMAP_ON)];
const DIVIDERS: &[Answer] = &[answer(TAILOR_OFF, TAILOR_DIVIDERS_OFF), answer(TAILOR_ON, TAILOR_DIVIDERS_ON)];
const PANEL: &[Answer] = &[
    answer(TAILOR_PANEL_CLOSED, TAILOR_PANEL_CLOSED_DETAIL),
    answer(TAILOR_PANEL_OPEN, TAILOR_PANEL_OPEN_DETAIL),
];
const TOOLS: &[Answer] =
    &[answer(TAILOR_TOOLS_FEW, TAILOR_TOOLS_FEW_DETAIL), answer(TAILOR_TOOLS_ALL, TAILOR_TOOLS_ALL_DETAIL)];

impl Question {
    /// In the order they are asked.
    pub const ALL: [Question; 7] = [
        Question::Start,
        Question::TitleRow,
        Question::StatusBar,
        Question::Minimap,
        Question::Dividers,
        Question::Panel,
        Question::Tools,
    ];

    pub fn title(self) -> Key {
        match self {
            Question::Start => TAILOR_START,
            Question::TitleRow => TAILOR_TITLE_ROW,
            Question::StatusBar => TAILOR_STATUS_BAR,
            Question::Minimap => TAILOR_MINIMAP,
            Question::Dividers => TAILOR_DIVIDERS,
            Question::Panel => TAILOR_PANEL,
            Question::Tools => TAILOR_TOOLS,
        }
    }

    /// Its answers, the least shown first.
    pub fn answers(self) -> &'static [Answer] {
        match self {
            Question::Start => START,
            Question::TitleRow => TITLE_ROW,
            Question::StatusBar => STATUS_BAR,
            Question::Minimap => MINIMAP,
            Question::Dividers => DIVIDERS,
            Question::Panel => PANEL,
            Question::Tools => TOOLS,
        }
    }

    /// Which answer `shown` gives, if one of them is it (a title row of the person's own is none).
    pub fn picked(self, shown: &Shown) -> Option<usize> {
        let flag = |on: bool| Some(usize::from(on));
        match self {
            Question::Start => {
                shown.preset().and_then(|preset| Preset::ALL.iter().position(|p| *p == preset))
            }
            Question::TitleRow => TitleShown::OFFERED.iter().position(|title| *title == shown.title),
            Question::StatusBar => flag(shown.status_bar),
            Question::Minimap => flag(shown.minimap),
            Question::Dividers => flag(shown.column_dividers),
            Question::Panel => flag(shown.panel_open),
            Question::Tools => match shown.tools {
                ToolsShown::Few => Some(0),
                ToolsShown::All => Some(1),
                ToolsShown::Own(_) => None,
            },
        }
    }

    /// `shown` with answer `ix` given; an answer past the end changes nothing.
    pub fn pick(self, ix: usize, shown: &Shown) -> Shown {
        let mut next = shown.clone();
        let on = ix == 1;
        match self {
            Question::Start => {
                if let Some(preset) = Preset::ALL.get(ix) {
                    next = Shown::of(*preset);
                }
            }
            Question::TitleRow => {
                if let Some(title) = TitleShown::OFFERED.get(ix) {
                    next.title = title.clone();
                }
            }
            _ if ix > 1 => {}
            Question::StatusBar => next.status_bar = on,
            Question::Minimap => next.minimap = on,
            Question::Dividers => next.column_dividers = on,
            Question::Panel => next.panel_open = on,
            Question::Tools => next.tools = if on { ToolsShown::All } else { ToolsShown::Few },
        }
        next
    }

    /// The window as answer `ix` would draw it on this question's card: [`Question::pick`], except that
    /// the tools question previews the panel open, as its tabs only show there.
    pub fn preview(self, ix: usize, shown: &Shown) -> Shown {
        let mut next = self.pick(ix, shown);
        if self == Question::Tools {
            next.panel_open = true;
        }
        next
    }
}

/// The question asked now, and its place ("2 of 7").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub question: Question,
    /// From 1.
    pub number: usize,
    pub of: usize,
}

/// Where the walk is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    Asking(Question),
    /// Past the last question: the summary.
    Done,
}

/// The walk: where it is, and what the answers so far make of the window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Walk {
    pub at: At,
    pub shown: Shown,
    /// What the window showed before the walk, for Start over.
    before: Shown,
}

impl Walk {
    /// A walk from what the window shows now.
    pub fn new(shown: Shown) -> Self {
        Self { at: At::Asking(Question::Start), before: shown.clone(), shown }
    }

    /// The question asked now and its place, or `None` on the summary.
    pub fn step(&self) -> Option<Step> {
        let At::Asking(question) = self.at else { return None };
        let ix = Question::ALL.iter().position(|q| *q == question).unwrap_or_default();
        Some(Step { question, number: ix + 1, of: Question::ALL.len() })
    }

    /// What the window showed before the walk: what Start over puts back.
    pub fn before(&self) -> &Shown {
        &self.before
    }

    /// Give answer `ix` to the question asked now and go on to the next one (after the last, the
    /// summary). Returns what the window shows now, to apply; `None` if nothing is asked or `ix`
    /// is no answer.
    pub fn answer(&mut self, ix: usize) -> Option<&Shown> {
        let Step { question, number, .. } = self.step()?;
        if ix >= question.answers().len() {
            return None;
        }
        self.shown = question.pick(ix, &self.shown);
        self.at = Question::ALL.get(number).map_or(At::Done, |next| At::Asking(*next));
        Some(&self.shown)
    }

    /// Back to the question before (from the summary, the last one). The answers stay: the
    /// question shows the one given. Returns whether it moved.
    pub fn back(&mut self) -> bool {
        let to = match self.at {
            At::Done => Question::ALL.last().copied(),
            At::Asking(question) => {
                let ix = Question::ALL.iter().position(|q| *q == question).unwrap_or_default();
                ix.checked_sub(1).map(|before| Question::ALL[before])
            }
        };
        let Some(to) = to else { return false };
        self.at = At::Asking(to);
        true
    }

    /// From the first question again, with the window as it was before the walk. Returns what to
    /// apply.
    pub fn start_over(&mut self) -> &Shown {
        self.at = At::Asking(Question::Start);
        self.shown = self.before.clone();
        &self.shown
    }

    /// The summary's lines: each question and the answer it has now (`None` for a title row of
    /// the person's own, which no answer describes).
    pub fn summary(&self) -> Vec<(Question, Option<Answer>)> {
        Question::ALL
            .into_iter()
            .filter(|q| *q != Question::Start)
            .map(|q| (q, q.picked(&self.shown).map(|ix| q.answers()[ix])))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{At, Question, Step, Walk};
    use crate::config::presets::{Preset, Shown, TitleShown, ToolsShown};

    fn balanced() -> Walk {
        Walk::new(Shown::of(Preset::Balanced))
    }

    #[test]
    fn a_preset_answers_every_question_and_the_next_ones_show_its_answers() {
        let mut walk = balanced();
        assert_eq!(walk.step(), Some(Step { question: Question::Start, number: 1, of: 7 }));
        assert_eq!(Question::Start.picked(&walk.shown), Some(1), "Balanced, as the defaults are");
        assert_eq!(walk.answer(0), Some(&Shown::of(Preset::Zen)));
        assert_eq!(walk.step().map(|step| step.question), Some(Question::TitleRow));
        assert_eq!(Question::TitleRow.picked(&walk.shown), Some(0), "just the name");
        assert_eq!(Question::Panel.picked(&walk.shown), Some(0), "closed");
    }

    #[test]
    fn each_question_changes_its_own_part_and_the_last_leads_to_the_summary() {
        let mut walk = balanced();
        walk.answer(0);
        walk.answer(2); // the whole title row
        walk.answer(1); // the status bar on
        for _ in 0..4 {
            walk.answer(0);
        }
        assert_eq!(walk.at, At::Done);
        let shown = &walk.shown;
        assert_eq!(shown.title, TitleShown::Everything);
        assert!(shown.status_bar && !shown.minimap && !shown.column_dividers && !shown.panel_open);
        assert_eq!(shown.tools, ToolsShown::Few);
        assert_eq!(shown.preset(), None, "no preset any more");
        assert_eq!(walk.summary().len(), 6);
        assert!(walk.answer(0).is_none(), "nothing is asked on the summary");
    }

    #[test]
    fn back_keeps_the_answers_and_start_over_brings_back_the_window_as_it_was() {
        let mut walk = balanced();
        assert!(!walk.back(), "nothing before the first question");
        walk.answer(2);
        walk.answer(0);
        assert!(walk.back());
        assert_eq!(walk.step().map(|step| step.question), Some(Question::TitleRow));
        assert_eq!(Question::TitleRow.picked(&walk.shown), Some(0), "the answer given stays");
        assert!(walk.answer(9).is_none(), "no ninth answer");
        assert_eq!(walk.start_over(), &Shown::of(Preset::Balanced));
        assert_eq!(walk.before(), &Shown::of(Preset::Balanced));
        assert_eq!(walk.at, At::Asking(Question::Start));
    }

    /// Two answers of a question that draw the same window are no choice to look at.
    #[test]
    fn no_two_answers_of_a_question_preview_the_same_window() {
        for preset in Preset::ALL {
            let shown = Shown::of(preset);
            for question in Question::ALL {
                let previews: Vec<Shown> =
                    (0..question.answers().len()).map(|ix| question.preview(ix, &shown)).collect();
                for (a, first) in previews.iter().enumerate() {
                    for second in &previews[a + 1..] {
                        assert_ne!(first, second, "{question:?} from {preset:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn every_question_offers_two_or_three_answers_and_each_reads_back() {
        let shown = Shown::of(Preset::Balanced);
        for question in Question::ALL {
            let count = question.answers().len();
            assert!((2..=3).contains(&count), "{question:?}");
            for ix in 0..count {
                assert_eq!(question.picked(&question.pick(ix, &shown)), Some(ix), "{question:?} {ix}");
            }
        }
    }
}

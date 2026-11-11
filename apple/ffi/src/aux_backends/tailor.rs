//! "How much to show", the Look step's walk through what the window displays (core's `onboarding::tailor`): one
//! question at a time, its answers drawn as the window each would give, Back and Start over, a summary and "Looks
//! good". The walk, what each answer means and the words are core's; this holds the walk of one run, writes each
//! answer to the Default profile's `config.json` as it is given (only what the answer changed), and lays out the
//! window sketches the answers are drawn as (the GPUI app's `style/window_sketch.rs`, which cannot be linked from
//! here: the layout is the same pure function over the same `Shown`).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use wormhole_core::config::Config;
use wormhole_core::config::presets::{Shown, ToolsShown, offered_tools};
use wormhole_core::onboarding::tailor::{At, Walk};
use wormhole_core::storage::paths::DataRoot;
use wormhole_core::view::layout::title_parts::PartKind;
use wormhole_text::keys::{
    TAILOR_ALL_SET, TAILOR_BACK, TAILOR_HEADING, TAILOR_LOOKS_GOOD, TAILOR_OWN, TAILOR_START_OVER,
    TAILOR_SUMMARY, tailor_announce, tailor_counter,
};
use wormhole_text::text;

use super::common::{AuxError, locked, resolve_data_dir};

/// How tall a sketch is: four lines of text and the bars around them.
const SKETCH_H: f32 = 80.0;
const SIDEBAR_W: f32 = 20.0;
const PANEL_W: f32 = 28.0;
const MINIMAP_W: f32 = 4.0;
const TITLE_H: f32 = 12.0;
const STATUS_H: f32 = 8.0;
const BAR: f32 = 3.0;
const GAP: f32 = 2.0;
const PAD: f32 = 4.0;
const RULE: f32 = 1.0;
const CRUMB_W: f32 = 8.0;
const INLINE_W: f32 = 3.0;
const CHIP_W: f32 = 5.0;
const BUTTON_W: f32 = 5.0;
const LINES: [f32; 4] = [0.8, 0.55, 0.7, 0.35];
const TABS_ALWAYS: usize = 2;
const TABS_MAX: usize = 4;
const ROWS: usize = 3;

/// What a block of a sketch is made of, by role; the view maps each to a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SketchFill {
    Sidebar,
    PanelRight,
    Rule,
    SelectedRow,
    Faint,
    Muted,
    Chip,
    Primary,
    Tab,
    Minimap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SketchCorner {
    Square,
    /// The theme's small radius.
    Soft,
    /// As round as the block's thinner side allows.
    Round,
}

/// One block to paint, in paint order, in points from the sketch's top left.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct SketchPiece {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub fill: SketchFill,
    pub corner: SketchCorner,
}

fn piece(x: f32, y: f32, w: f32, h: f32, fill: SketchFill, corner: SketchCorner) -> SketchPiece {
    SketchPiece { x, y, w, h, fill, corner }
}

/// Where every block of a sketch `w` by `h` points goes for `shown`. `tools` is how many optional tools this build
/// offers (so that "all of them" draws as many tabs). Blocks that would not fit are left out.
fn pieces(shown: &Shown, tools: usize, w: f32, h: f32) -> Vec<SketchPiece> {
    use SketchCorner::{Round, Soft, Square};
    use SketchFill::{Faint, Minimap, Muted, PanelRight, Primary, Rule, SelectedRow, Sidebar, Tab};
    let more = match &shown.tools {
        ToolsShown::Few => 0,
        ToolsShown::All => tools,
        ToolsShown::Own(on) => on.len(),
    };
    let panel_tabs = shown.panel_open.then_some((TABS_ALWAYS + more).min(TABS_MAX));
    let mut out = Vec::with_capacity(40);
    let body_h = (h - if shown.status_bar { STATUS_H } else { 0.0 }).max(0.0);
    let panel_w = if panel_tabs.is_some() { PANEL_W } else { 0.0 };
    let main_right = w - panel_w;

    out.push(piece(0.0, 0.0, SIDEBAR_W, body_h, Sidebar, Square));
    if let Some(tabs) = panel_tabs {
        out.push(piece(main_right, 0.0, panel_w, body_h, PanelRight, Square));
        for tab in 0..tabs {
            let x = main_right + PAD + tab as f32 * (BAR + GAP);
            out.push(piece(x, PAD, BAR, BAR, if tab == 0 { Tab } else { Faint }, Round));
        }
        for (line, share) in [0.8, 0.55].into_iter().enumerate() {
            let y = PAD + (line + 1) as f32 * (BAR + GAP);
            out.push(piece(main_right + PAD, y, (panel_w - 2.0 * PAD) * share, BAR, Faint, Round));
        }
    }
    if shown.column_dividers {
        out.push(piece(SIDEBAR_W - RULE, 0.0, RULE, body_h, Rule, Square));
        if panel_tabs.is_some() {
            out.push(piece(main_right, 0.0, RULE, body_h, Rule, Square));
        }
    }
    for row in 0..ROWS {
        let y = PAD + row as f32 * (BAR + GAP);
        out.push(piece(PAD, y, SIDEBAR_W - 2.0 * PAD, BAR, if row == 0 { SelectedRow } else { Faint }, Soft));
    }
    title_parts(shown, main_right, &mut out);

    let text_x = SIDEBAR_W + PAD;
    let minimap_w = if shown.minimap { MINIMAP_W + GAP } else { 0.0 };
    let text_w = (main_right - text_x - PAD - minimap_w).max(0.0);
    for (line, share) in LINES.iter().enumerate() {
        let y = TITLE_H + PAD + line as f32 * (BAR + GAP);
        if y + BAR > body_h {
            break;
        }
        out.push(piece(text_x, y, text_w * share, BAR, if line == 0 { Primary } else { Faint }, Round));
    }
    if shown.minimap {
        out.push(piece(main_right - MINIMAP_W - GAP, TITLE_H + GAP, MINIMAP_W, body_h - TITLE_H - 2.0 * GAP, Minimap, Soft));
    }
    if shown.status_bar {
        let bar_y = body_h + (STATUS_H - BAR) / 2.0;
        out.push(piece(0.0, body_h, w, STATUS_H, Sidebar, Square));
        out.push(piece(0.0, body_h, w, RULE, Rule, Square));
        out.push(piece(PAD, bar_y, CRUMB_W, BAR, Muted, Round));
        out.push(piece(w - PAD - CRUMB_W, bar_y, CRUMB_W, BAR, Faint, Round));
    }
    out
}

/// The title row's parts between the sidebar and `right`, in order from the left, the space between the row's two
/// ends taking what is left.
fn title_parts(shown: &Shown, right: f32, out: &mut Vec<SketchPiece>) {
    let kinds: Vec<PartKind> = shown.title.row().parts().iter().map(|part| part.kind()).collect();
    let width = |kind: PartKind| match kind {
        PartKind::Crumb => CRUMB_W,
        PartKind::Inline => INLINE_W,
        PartKind::Chip => CHIP_W,
        PartKind::Button => BUTTON_W,
        PartKind::Space => 0.0,
    };
    let spaces = kinds.iter().filter(|kind| **kind == PartKind::Space).count();
    let fixed: f32 = kinds.iter().map(|kind| width(*kind)).sum();
    let gaps = kinds.len().saturating_sub(1) as f32 * GAP;
    let free = (right - SIDEBAR_W - 2.0 * PAD - fixed - gaps).max(0.0);
    let space_w = if spaces == 0 { 0.0 } else { free / spaces as f32 };
    let y = (TITLE_H - BAR) / 2.0;
    let mut x = SIDEBAR_W + PAD;
    for kind in kinds {
        let w = if kind == PartKind::Space { space_w } else { width(kind) };
        if x + w > right - PAD + f32::EPSILON {
            break;
        }
        let fill = match kind {
            PartKind::Crumb => Some(SketchFill::Muted),
            PartKind::Inline | PartKind::Button => Some(SketchFill::Faint),
            PartKind::Chip => Some(SketchFill::Chip),
            PartKind::Space => None,
        };
        if let Some(fill) = fill {
            out.push(piece(x, y, w, BAR, fill, SketchCorner::Round));
        }
        x += w + GAP;
    }
}

/// An answer a question offers, with the window it would give.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct TailorAnswerRecord {
    pub label: String,
    pub detail: String,
    /// The one the window shows now.
    pub picked: bool,
    pub sketch: Vec<SketchPiece>,
}

/// A line of the summary: a question and the answer it has now (`None`: a title row of the person's own).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TailorLineRecord {
    pub question: String,
    pub answer: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TailorStage {
    Asking,
    Summary,
    /// "Looks good" was pressed: the walk is a line, with Start over.
    Settled,
}

/// What the walk shows now.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct TailorRecord {
    pub stage: TailorStage,
    pub heading: String,
    pub question: String,
    /// "2 of 7"; empty on the summary.
    pub counter: String,
    pub answers: Vec<TailorAnswerRecord>,
    /// The line about the answer in use.
    pub detail: String,
    pub summary: Vec<TailorLineRecord>,
    pub summary_title: String,
    pub settled_text: String,
    pub back: String,
    pub start_over: String,
    pub looks_good: String,
    pub can_back: bool,
    /// What a screen reader is told as the walk moves.
    pub announcement: String,
    /// Whether the window's right panel is to be open (the window's own, so the app applies it).
    pub panel_open: bool,
}

/// What the person does in the walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TailorActRecord {
    /// Answer number `n` (from 0) of the question asked.
    Answer { n: u32 },
    Back,
    StartOver,
    /// "Looks good" on the summary.
    Settle,
}

struct Run {
    walk: Walk,
    settled: bool,
}

#[derive(uniffi::Object)]
pub struct OnboardingTailor {
    data_dir: PathBuf,
    run: Mutex<Run>,
}

#[uniffi::export]
impl OnboardingTailor {
    /// A walk from what the Default profile's settings show now and the window's panel (`panel_open`).
    #[uniffi::constructor]
    pub fn new(panel_open: bool, data_dir: Option<String>) -> Result<Arc<Self>, AuxError> {
        let data_dir = resolve_data_dir(data_dir)?;
        let shown = Shown::read(&Self::settings(&data_dir), panel_open);
        Ok(Arc::new(Self { data_dir, run: Mutex::new(Run { walk: Walk::new(shown), settled: false }) }))
    }

    /// Whether the act changed anything (an answer past the last, Back on the first question and Looks good off the
    /// summary do nothing). What an answer changes is written to `config.json` at once, and only that.
    pub fn act(&self, act: TailorActRecord) -> bool {
        let mut run = locked(&self.run);
        let from = run.walk.shown.clone();
        let moved = match act {
            TailorActRecord::Answer { n } => run.walk.answer(n as usize).is_some(),
            TailorActRecord::Back => run.walk.back(),
            TailorActRecord::StartOver => {
                run.settled = false;
                run.walk.start_over();
                true
            }
            TailorActRecord::Settle => {
                run.settled = run.walk.at == At::Done;
                run.settled
            }
        };
        if !moved {
            return false;
        }
        let shown = run.walk.shown.clone();
        drop(run);
        if shown != from {
            let profile = DataRoot::new(&self.data_dir).default_profile();
            Config::load_for(&profile).update(|settings| shown.apply_changes(&from, settings));
        }
        true
    }

    /// The question (or summary) as it is now, with each answer's window sketched `width` points wide.
    pub fn view(&self, width: f32) -> TailorRecord {
        let run = locked(&self.run);
        let tools = offered_tools(&Self::settings(&self.data_dir)).len();
        let walk = &run.walk;
        let stage = if run.settled {
            TailorStage::Settled
        } else if walk.step().is_some() {
            TailorStage::Asking
        } else {
            TailorStage::Summary
        };
        let (mut question, mut counter, mut answers, mut detail) = (String::new(), String::new(), Vec::new(), String::new());
        let mut announcement = text(TAILOR_SUMMARY).to_string();
        if let Some(step) = walk.step() {
            let asked = step.question;
            let picked = asked.picked(&walk.shown);
            question = text(asked.title()).to_string();
            counter = tailor_counter(&step.number.to_string(), &step.of.to_string());
            announcement = tailor_announce(&question, &step.number.to_string(), &step.of.to_string());
            answers = asked
                .answers()
                .iter()
                .enumerate()
                .map(|(ix, answer)| TailorAnswerRecord {
                    label: text(answer.label).to_string(),
                    detail: text(answer.detail).to_string(),
                    picked: picked == Some(ix),
                    sketch: pieces(&asked.preview(ix, &walk.shown), tools, width, SKETCH_H),
                })
                .collect();
            detail = picked.map(|ix| text(asked.answers()[ix].detail).to_string()).unwrap_or_default();
        }
        if stage == TailorStage::Settled {
            announcement = text(TAILOR_ALL_SET).to_string();
        }
        TailorRecord {
            stage,
            heading: text(TAILOR_HEADING).to_string(),
            question,
            counter,
            answers,
            detail,
            summary: walk
                .summary()
                .into_iter()
                .map(|(question, answer)| TailorLineRecord {
                    question: text(question.title()).to_string(),
                    answer: answer.map_or_else(|| text(TAILOR_OWN).to_string(), |a| text(a.label).to_string()),
                })
                .collect(),
            summary_title: text(TAILOR_SUMMARY).to_string(),
            settled_text: text(TAILOR_ALL_SET).to_string(),
            back: text(TAILOR_BACK).to_string(),
            start_over: text(TAILOR_START_OVER).to_string(),
            looks_good: text(TAILOR_LOOKS_GOOD).to_string(),
            can_back: walk.step().is_none_or(|step| step.number > 1),
            announcement,
            panel_open: walk.shown.panel_open,
        }
    }
}

impl OnboardingTailor {
    fn settings(data_dir: &std::path::Path) -> wormhole_core::config::Settings {
        Config::load_for(&DataRoot::new(data_dir).default_profile()).settings
    }
}

/// A sketch of `shown` as the window would draw it (the Studio gallery's specimen), `width` points wide.
#[uniffi::export]
pub fn window_sketch(preset: u32, width: f32) -> Vec<SketchPiece> {
    use wormhole_core::config::presets::Preset;
    let shown = Preset::ALL.get(preset as usize).map_or_else(|| Shown::of(Preset::Balanced), |p| Shown::of(*p));
    pieces(&shown, TABS_MAX - TABS_ALWAYS, width, SKETCH_H)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wormhole_core::config::presets::Preset;

    /// The walk over a scratch data folder: the first question's answers are the presets, an answer is written to
    /// `config.json` as it is given (and only what it changes), Back and Start over put things back.
    #[test]
    fn an_answer_is_written_as_it_is_given_and_start_over_puts_it_back() {
        crate::isolate_for_tests();
        let dir = tempfile::tempdir().expect("temp");
        let data = dir.path().join("Wormhole-swift-tailor");
        std::fs::create_dir_all(&data).expect("data dir");
        let tailor = OnboardingTailor::new(false, Some(data.display().to_string())).expect("a walk");
        let first = tailor.view(118.0);
        assert_eq!(first.stage, TailorStage::Asking);
        assert_eq!(first.answers.len(), 3, "Zen, Balanced, Everything");
        assert!(first.answers.iter().all(|a| !a.sketch.is_empty()));
        assert!(!first.can_back, "nothing before the first question");

        assert!(tailor.act(TailorActRecord::Answer { n: 2 }), "Everything");
        let settings = || Config::load_for(&DataRoot::new(&data).default_profile()).settings;
        assert!(settings().status_bar && settings().minimap, "written at once");
        assert!(tailor.view(118.0).panel_open, "the panel is the window's own: told, not written");
        assert!(tailor.act(TailorActRecord::Back) && tailor.view(118.0).question == first.question);
        assert!(tailor.act(TailorActRecord::StartOver));
        assert!(!settings().status_bar && !settings().minimap, "the window as it was before the walk");
        assert!(!tailor.act(TailorActRecord::Settle), "nothing to accept before the summary");
        assert!(!tailor.act(TailorActRecord::Answer { n: 9 }), "no ninth answer");
    }

    #[test]
    fn every_block_is_inside_the_sketch_and_a_preset_draws_what_it_shows() {
        for preset in Preset::ALL {
            let blocks = pieces(&Shown::of(preset), 2, 118.0, SKETCH_H);
            for b in &blocks {
                assert!(b.x >= 0.0 && b.y >= 0.0 && b.x + b.w <= 118.001 && b.y + b.h <= SKETCH_H + 0.001, "{preset:?} {b:?}");
            }
        }
        let has = |preset, fill| pieces(&Shown::of(preset), 2, 118.0, SKETCH_H).iter().any(|b| b.fill == fill);
        assert!(!has(Preset::Zen, SketchFill::Minimap) && !has(Preset::Zen, SketchFill::PanelRight));
        assert!(has(Preset::Everything, SketchFill::Minimap) && has(Preset::Everything, SketchFill::PanelRight));
    }
}

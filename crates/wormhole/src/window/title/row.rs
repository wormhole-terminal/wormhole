//! The title row laid out as the person arranged it (`title_row`, core's `layout::title_parts`):
//! the parts before the flexible space at the row's left end, the others at its right end, each
//! only while it has something to show. Two crumbs side by side are joined by a chevron, buttons
//! side by side stand flush, and everything else stands a gap apart.
//!
//! This works out what each end holds, the room its fixed pieces take (what `fit_row` counts as
//! `fixed`), and draws the ends from the parts that `breadcrumb`, `chips` and the buttons
//! (`window::title`) build. The end that holds the terminal's name (else the left one, else the
//! right) is the row's keyboard stop.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::focus::Area;
use wormhole_core::view::layout::title_parts::{PartKind, TitlePart, chevrons};
use wormhole_core::view::layout::title_row::Extra;

use crate::settings::ui_in;
use crate::style;
use crate::window::Workspace;
use crate::window::title::breadcrumb::{CRUMB_GAP, CRUMB_PAD};
use crate::window::title::{AFTER_CHEVRON, Facts, ROW_GAP, TitlePlan};

/// One end of the row: its parts in order, each with whether a chevron comes before it.
#[derive(Debug, Default, PartialEq)]
pub(in crate::window) struct End {
    pub(in crate::window) parts: Vec<TitlePart>,
    pub(in crate::window) chevrons: Vec<bool>,
}

impl End {
    fn of(parts: Vec<TitlePart>) -> Self {
        let chevrons = chevrons(&parts);
        Self { parts, chevrons }
    }
}

/// The row's two ends for `facts`: the parts arranged at each that have something to show. A chevron
/// is decided on these, not on what the row has room for, so none comes and goes as the window
/// narrows.
pub(in crate::window) fn ends(facts: &Facts) -> (End, End) {
    let (left, right) = facts.row.ends();
    let present = |parts: &[TitlePart]| parts.iter().copied().filter(|part| has(facts, *part)).collect();
    (End::of(present(left)), End::of(present(right)))
}

/// Whether the row draws `part` for these facts: it is arranged in the row and has something to
/// show (a branch needs a repository, Open in… a folder). The extras' facts are only gathered when
/// they are arranged.
pub(in crate::window) fn has(facts: &Facts, part: TitlePart) -> bool {
    facts.row.shows(part)
        && match part {
            TitlePart::Folder | TitlePart::OpenIn => facts.folder.is_some(),
            TitlePart::Branch => facts.branch.is_some(),
            TitlePart::BranchStatus => facts.extras.branch.is_some(),
            TitlePart::TerminalStatus => facts.words.is_some(),
            TitlePart::AgentGauge => facts.extras.gauge.is_some(),
            TitlePart::Ports => !facts.extras.ports.is_empty(),
            TitlePart::PullRequest => facts.extras.pull_request.is_some(),
            TitlePart::Terminal | TitlePart::Splits | TitlePart::Space => true,
        }
}

/// The gap before `part`, after `before` (`None`: it starts its end, where a button stands flush and
/// anything else has the breadcrumb's padding).
fn gap(before: Option<TitlePart>, part: TitlePart) -> f32 {
    match (before.map(TitlePart::kind), part.kind()) {
        (None, PartKind::Button) => 0.0,
        (None, _) => CRUMB_PAD,
        // Open in…'s chevron is narrow: a little room after it, so its glyph stands from the next as far
        // as the row's other glyphs stand from each other.
        (Some(PartKind::Button), PartKind::Button) if before == Some(TitlePart::OpenIn) => AFTER_CHEVRON,
        (Some(PartKind::Button), PartKind::Button) => 0.0,
        (Some(PartKind::Button), _) | (Some(_), PartKind::Button) => ROW_GAP,
        _ => CRUMB_GAP,
    }
}

/// The room after an end's last part: the breadcrumb's padding, or none after a button.
fn trailing(last: Option<TitlePart>) -> f32 {
    match last.map(TitlePart::kind) {
        None | Some(PartKind::Button) => 0.0,
        Some(_) => CRUMB_PAD,
    }
}

/// The part's own extra that `fit_row` gives up, whose width already holds the gap before it.
fn extra(part: TitlePart) -> Option<Extra> {
    match part {
        TitlePart::BranchStatus => Some(Extra::BranchStatus),
        TitlePart::AgentGauge => Some(Extra::Gauge),
        TitlePart::Ports => Some(Extra::Ports),
        TitlePart::PullRequest => Some(Extra::PullRequest),
        _ => None,
    }
}

/// What never gives way on the ends: the gaps before the crumbs, the chevrons, the folder's icon,
/// the terminal's tile with its gap, and the padding. (The extras, the status words and the
/// buttons bring their gaps with their own widths.)
pub(in crate::window) fn fixed_width(ends: &(End, End)) -> f32 {
    [&ends.0, &ends.1]
        .into_iter()
        .map(|end| {
            let mut width = trailing(end.parts.last().copied());
            for (ix, part) in end.parts.iter().enumerate() {
                let brings_gap = extra(*part).is_some()
                    || part.kind() == PartKind::Button
                    || *part == TitlePart::TerminalStatus;
                if brings_gap {
                    continue;
                }
                width += gap(ix.checked_sub(1).map(|before| end.parts[before]), *part);
                if end.chevrons[ix] {
                    width += style::GLYPH + CRUMB_GAP;
                }
                width += match part {
                    TitlePart::Folder => style::GLYPH,
                    TitlePart::Terminal => style::TILE_TITLE + CRUMB_GAP,
                    _ => 0.0,
                };
            }
            width
        })
        .sum()
}

impl Workspace {
    /// The row's two ends, each a strip of its parts; the space between them is the window's title
    /// bar.
    pub(in crate::window) fn render_title_ends(
        &self,
        plan: &TitlePlan,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> (AnyElement, AnyElement) {
        let (left, right) = &plan.ends;
        // The keyboard stop goes where the name is, else where anything is.
        let stop = if right.parts.contains(&TitlePart::Terminal) || left.parts.is_empty() {
            Edge::Right
        } else {
            Edge::Left
        };
        let left = self.render_end(plan, left, stop == Edge::Left, window, cx);
        let right = self.render_end(plan, right, stop == Edge::Right, window, cx);
        (left, right)
    }

    /// One end: its parts in order, the ones the row had room for, with the gaps and chevrons between
    /// them. As the row's keyboard stop (`stop`) it wears the ring.
    fn render_end(
        &self,
        plan: &TitlePlan,
        end: &End,
        stop: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        let mut children: Vec<AnyElement> = Vec::new();
        let mut before = None;
        for (part, chevron) in end.parts.iter().zip(&end.chevrons) {
            let Some(drawn) = self.render_title_part(plan, *part, cx) else { continue };
            let mut lead = gap(before, *part);
            if *chevron {
                children.push(
                    div()
                        .flex_none()
                        .ml(px(lead))
                        .text_color(ui.faint)
                        .child(style::icon(IconName::ChevronRight))
                        .into_any_element(),
                );
                lead = CRUMB_GAP;
            }
            let slot = div().flex().items_center().ml(px(lead));
            // The name is what gives way inside an end: it is cut at its end; everything else keeps its
            // width (`fit_row` decided which of them there is room for).
            let slot = if *part == TitlePart::Terminal { slot.min_w_0() } else { slot.flex_none() };
            children.push(slot.children(drawn).into_any_element());
            before = Some(*part);
        }
        let pad = trailing(before);
        let ring = stop && self.ring(Area::Tabs, window);
        let side = div()
            .id(if stop { "title-strip" } else { "title-end" })
            // Clicks here are for the parts, not for dragging the window.
            .when(!children.is_empty(), |d| d.occlude())
            .relative()
            .flex()
            .items_center()
            .min_w_0()
            .pr(px(pad))
            .children(children)
            .when(ring, |d| d.child(style::focus_ring(ui, style::RingPlacement::Outside, ui.radius)));
        if !stop {
            return side.into_any_element();
        }
        // The probe the path menu opens under when there is no folder to open it under: ⌃↩ then
        // opens the row's own menu.
        let side = side.when(!plan.ends_have(TitlePart::Folder), |d| d.child(self.path_anchor.probe()));
        self.area_container(Area::Tabs, side, cx).into_any_element()
    }

    /// What one part draws, if the row has room for it now.
    fn render_title_part(
        &self,
        plan: &TitlePlan,
        part: TitlePart,
        cx: &mut Context<Self>,
    ) -> Option<Vec<AnyElement>> {
        let one = |element: Option<AnyElement>| element.map(|element| vec![element]);
        match part {
            TitlePart::Folder => one(self.render_folder_crumb(plan, cx)),
            TitlePart::Branch => self.render_branch_crumb(plan, cx),
            TitlePart::BranchStatus => one(self.render_branch_status(plan, cx)),
            TitlePart::Terminal => one(Some(self.render_terminal_crumb(plan, cx))),
            TitlePart::TerminalStatus => one(self.render_terminal_words(plan, cx)),
            TitlePart::AgentGauge => one(self.render_gauge_chip(plan, cx)),
            TitlePart::Ports => self.render_port_chips(plan, cx),
            TitlePart::PullRequest => one(self.render_pull_request_chip(plan, cx)),
            TitlePart::OpenIn | TitlePart::Splits => self.render_title_buttons(plan, part, cx),
            TitlePart::Space => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Edge {
    Left,
    Right,
}

#[cfg(test)]
mod tests {
    use wormhole_core::view::layout::title_parts::TitlePart::*;

    use super::{End, fixed_width, gap};
    use crate::style;
    use crate::window::title::breadcrumb::{CRUMB_GAP, CRUMB_PAD};
    use crate::window::title::{AFTER_CHEVRON, ROW_GAP};

    #[test]
    fn crumbs_stand_a_gap_apart_and_buttons_flush() {
        assert_eq!(gap(None, Folder), CRUMB_PAD);
        assert_eq!(gap(None, Splits), 0.0);
        assert_eq!(gap(Some(Folder), Branch), CRUMB_GAP);
        assert_eq!(gap(Some(PullRequest), OpenIn), ROW_GAP);
        assert_eq!(gap(Some(OpenIn), Splits), AFTER_CHEVRON);
        assert_eq!(gap(Some(Splits), OpenIn), 0.0);
        assert_eq!(gap(Some(Splits), Terminal), ROW_GAP);
    }

    /// The default breadcrumb takes what it took before the row could be arranged: its padding, the
    /// folder's icon, two chevrons with a gap on each side, and the terminal's tile with its gap.
    #[test]
    fn the_default_breadcrumb_keeps_its_fixed_width() {
        let left = End::of(vec![Folder, Branch, BranchStatus, Terminal, TerminalStatus]);
        let right = End::of(vec![AgentGauge, Ports, PullRequest, OpenIn, Splits]);
        let before = 2.0 * CRUMB_PAD
            + style::GLYPH
            + 2.0 * (2.0 * CRUMB_GAP + style::GLYPH)
            + style::TILE_TITLE
            + CRUMB_GAP;
        assert_eq!(fixed_width(&(left, right)), before);
    }

    #[test]
    fn an_end_of_buttons_alone_takes_nothing_of_its_own() {
        let left = End::of(vec![Terminal]);
        let right = End::of(vec![OpenIn, Splits]);
        assert_eq!(fixed_width(&(left, right)), 2.0 * CRUMB_PAD + style::TILE_TITLE + CRUMB_GAP);
    }
}

//! A window drawn as a sketch: the columns, the title row's parts, the status bar and the minimap, each
//! as a few blocks. It shows what a [`Shown`] makes of the main window, so the first run's questions can
//! offer each answer as the window it would give. It is drawn from the real values (the title row's own
//! parts, whether the lines between the columns show), never a picture of them, and in the live theme's
//! colours.
//!
//! The sketch is one canvas, not a tree of boxes: [`pieces`] works out where every block goes (a pure
//! function, tested) and the canvas paints them, snapped to device pixels. A tree of some forty boxes
//! per sketch, three to a question, was most of what the first run's Look step cost a frame.

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::presets::{Shown, ToolsShown};
use wormhole_core::view::layout::title_parts::PartKind;

use crate::settings::UiColors;
use crate::style::Edges;

/// How tall the sketch is: tall enough for four lines of text and the bars around them.
const SKETCH_H: f32 = 80.0;
/// The sketch's columns and strips, in points of the sketch.
const SIDEBAR_W: f32 = 20.0;
/// Wide enough for the four tabs the panel shows at most.
const PANEL_W: f32 = 28.0;
const MINIMAP_W: f32 = 4.0;
const TITLE_H: f32 = 12.0;
const STATUS_H: f32 = 8.0;
/// A block of the sketch is this thick (a line of text, a tab, a part of the title row).
const BAR: f32 = 3.0;
const GAP: f32 = 2.0;
const PAD: f32 = 4.0;
/// The line between two columns, and above the status bar.
const RULE: f32 = 1.0;
/// The widths of the title row's parts, by how the row joins them.
const CRUMB_W: f32 = 8.0;
const INLINE_W: f32 = 3.0;
const CHIP_W: f32 = 5.0;
const BUTTON_W: f32 = 5.0;
/// The terminal's lines of text, as shares of the line.
const LINES: [f32; 4] = [0.8, 0.55, 0.7, 0.35];
/// The right panel's tabs: the two that are always there, and the most it draws.
const TABS_ALWAYS: usize = 2;
const TABS_MAX: usize = 4;
/// The sidebar's rows.
const ROWS: usize = 3;
/// How strong the chips and the minimap are drawn against the window.
const CHIP_ALPHA: f32 = 0.5;
const MINIMAP_ALPHA: f32 = 0.5;

/// The main window as [`Shown`] makes it, drawn small and as wide as its parent. The sidebar is always
/// there; the title row, the minimap, the lines between the columns, the status bar and the right panel
/// (a tab for each tool) follow `shown`. `tools_offered` is how many optional tools this build has, so
/// that "all of them" draws as many tabs.
#[track_caller]
pub fn window_sketch(ui: UiColors, shown: &Shown, tools_offered: usize) -> Div {
    let spec = Spec::of(shown, tools_offered);
    div()
        .w_full()
        .h(px(SKETCH_H))
        // The corner of a sketch inside a card is the small one: the card's own, less its inset.
        .rounded(ui.radius_sm)
        .overflow_hidden()
        .outlined(ui, ui.border)
        .bg(ui.background)
        .child(
            canvas(
                move |bounds, _, _| {
                    pieces(&spec, f32::from(bounds.size.width), f32::from(bounds.size.height))
                },
                move |bounds, pieces, window, _| paint(ui, bounds, &pieces, window),
            )
            .size_full(),
        )
}

/// What a sketch draws, from a [`Shown`].
#[derive(Clone, Debug, PartialEq, Eq)]
struct Spec {
    /// The title row's parts, left to right.
    title: Vec<PartKind>,
    status_bar: bool,
    minimap: bool,
    dividers: bool,
    /// How many tabs the right panel has; `None` while it is closed.
    panel_tabs: Option<usize>,
}

impl Spec {
    fn of(shown: &Shown, tools_offered: usize) -> Self {
        let more = match &shown.tools {
            ToolsShown::Few => 0,
            ToolsShown::All => tools_offered,
            ToolsShown::Own(on) => on.len(),
        };
        Self {
            title: shown.title.row().parts().iter().map(|part| part.kind()).collect(),
            status_bar: shown.status_bar,
            minimap: shown.minimap,
            dividers: shown.column_dividers,
            panel_tabs: shown.panel_open.then_some((TABS_ALWAYS + more).min(TABS_MAX)),
        }
    }
}

/// A rectangle of the sketch, in points from its top left.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

const fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

/// What a block is made of, by role; [`color`] says which colour of the theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fill {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Corner {
    Square,
    /// The theme's small radius.
    Soft,
    /// As round as the block's thinner side allows.
    Round,
}

/// One block to paint, in paint order.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Piece {
    at: Rect,
    fill: Fill,
    corner: Corner,
}

const fn piece(at: Rect, fill: Fill, corner: Corner) -> Piece {
    Piece { at, fill, corner }
}

/// Where every block of a sketch `w` by `h` points goes. Blocks that would not fit are left out, as a
/// clipped box would not show them.
fn pieces(spec: &Spec, w: f32, h: f32) -> Vec<Piece> {
    let mut out = Vec::with_capacity(40);
    let body_h = (h - if spec.status_bar { STATUS_H } else { 0.0 }).max(0.0);
    let panel_w = if spec.panel_tabs.is_some() { PANEL_W } else { 0.0 };
    let main_right = w - panel_w;

    // The columns, and the lines between them.
    out.push(piece(rect(0.0, 0.0, SIDEBAR_W, body_h), Fill::Sidebar, Corner::Square));
    if let Some(tabs) = spec.panel_tabs {
        out.push(piece(rect(main_right, 0.0, panel_w, body_h), Fill::PanelRight, Corner::Square));
        // Its tabs, the first current, and two lines of what it lists.
        for tab in 0..tabs {
            let x = main_right + PAD + tab as f32 * (BAR + GAP);
            let fill = if tab == 0 { Fill::Tab } else { Fill::Faint };
            out.push(piece(rect(x, PAD, BAR, BAR), fill, Corner::Round));
        }
        for (line, share) in [0.8, 0.55].into_iter().enumerate() {
            let y = PAD + (line + 1) as f32 * (BAR + GAP);
            let width = (panel_w - 2.0 * PAD) * share;
            out.push(piece(rect(main_right + PAD, y, width, BAR), Fill::Faint, Corner::Round));
        }
    }
    if spec.dividers {
        out.push(piece(rect(SIDEBAR_W - RULE, 0.0, RULE, body_h), Fill::Rule, Corner::Square));
        if spec.panel_tabs.is_some() {
            out.push(piece(rect(main_right, 0.0, RULE, body_h), Fill::Rule, Corner::Square));
        }
    }

    // The sidebar's rows, the first the current terminal.
    for row in 0..ROWS {
        let fill = if row == 0 { Fill::SelectedRow } else { Fill::Faint };
        let y = PAD + row as f32 * (BAR + GAP);
        out.push(piece(rect(PAD, y, SIDEBAR_W - 2.0 * PAD, BAR), fill, Corner::Soft));
    }

    title_parts(spec, SIDEBAR_W, main_right, &mut out);

    // The terminal's text, and the minimap along its right edge.
    let text_x = SIDEBAR_W + PAD;
    let minimap_w = if spec.minimap { MINIMAP_W + GAP } else { 0.0 };
    let text_w = (main_right - text_x - PAD - minimap_w).max(0.0);
    for (line, share) in LINES.iter().enumerate() {
        let y = TITLE_H + PAD + line as f32 * (BAR + GAP);
        if y + BAR > body_h {
            break;
        }
        let fill = if line == 0 { Fill::Primary } else { Fill::Faint };
        out.push(piece(rect(text_x, y, text_w * share, BAR), fill, Corner::Round));
    }
    if spec.minimap {
        let strip =
            rect(main_right - MINIMAP_W - GAP, TITLE_H + GAP, MINIMAP_W, body_h - TITLE_H - 2.0 * GAP);
        out.push(piece(strip, Fill::Minimap, Corner::Soft));
    }

    // The status bar along the bottom: its line, a name on the left and a size on the right.
    if spec.status_bar {
        let y = body_h;
        out.push(piece(rect(0.0, y, w, STATUS_H), Fill::Sidebar, Corner::Square));
        out.push(piece(rect(0.0, y, w, RULE), Fill::Rule, Corner::Square));
        let bar_y = y + (STATUS_H - BAR) / 2.0;
        out.push(piece(rect(PAD, bar_y, CRUMB_W, BAR), Fill::Muted, Corner::Round));
        out.push(piece(rect(w - PAD - CRUMB_W, bar_y, CRUMB_W, BAR), Fill::Faint, Corner::Round));
    }
    out
}

/// The title row's parts between `left` and `right`: in order from the left, the space between the
/// row's two ends taking what is left.
fn title_parts(spec: &Spec, left: f32, right: f32, out: &mut Vec<Piece>) {
    let width = |kind: PartKind| match kind {
        PartKind::Crumb => CRUMB_W,
        PartKind::Inline => INLINE_W,
        PartKind::Chip => CHIP_W,
        PartKind::Button => BUTTON_W,
        PartKind::Space => 0.0,
    };
    let spaces = spec.title.iter().filter(|kind| **kind == PartKind::Space).count();
    let fixed: f32 = spec.title.iter().map(|kind| width(*kind)).sum();
    let gaps = spec.title.len().saturating_sub(1) as f32 * GAP;
    let free = (right - left - 2.0 * PAD - fixed - gaps).max(0.0);
    let space_w = if spaces == 0 { 0.0 } else { free / spaces as f32 };
    let y = (TITLE_H - BAR) / 2.0;
    let mut x = left + PAD;
    for kind in &spec.title {
        let w = if *kind == PartKind::Space { space_w } else { width(*kind) };
        if x + w > right - PAD + f32::EPSILON {
            break;
        }
        let fill = match kind {
            PartKind::Crumb => Some(Fill::Muted),
            PartKind::Inline | PartKind::Button => Some(Fill::Faint),
            PartKind::Chip => Some(Fill::Chip),
            PartKind::Space => None,
        };
        if let Some(fill) = fill {
            out.push(piece(rect(x, y, w, BAR), fill, Corner::Round));
        }
        x += w + GAP;
    }
}

fn color(ui: UiColors, fill: Fill) -> Hsla {
    match fill {
        Fill::Sidebar => ui.panel,
        Fill::PanelRight => ui.panel_right,
        Fill::Rule => ui.divider,
        Fill::SelectedRow => ui.selected_row,
        Fill::Faint => ui.faint,
        Fill::Muted => ui.muted,
        Fill::Chip => ui.primary.opacity(CHIP_ALPHA),
        Fill::Primary => ui.primary,
        Fill::Tab => ui.selected_chip,
        Fill::Minimap => ui.faint.opacity(MINIMAP_ALPHA),
    }
}

/// Paint the blocks in `bounds`, each edge snapped to a device pixel so a one-point line stays one line.
fn paint(ui: UiColors, bounds: Bounds<Pixels>, pieces: &[Piece], window: &mut Window) {
    let scale = window.scale_factor();
    let snap = |points: f32| px((points * scale).round() / scale);
    let (left, top) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    for piece in pieces {
        let Rect { x, y, w, h } = piece.at;
        let corners = Bounds::from_corners(
            point(snap(left + x), snap(top + y)),
            point(snap(left + x + w), snap(top + y + h)),
        );
        let radius = match piece.corner {
            Corner::Square => px(0.0),
            Corner::Soft => ui.radius_sm,
            Corner::Round => px(w.min(h) / 2.0),
        };
        window.paint_quad(fill(corners, color(ui, piece.fill)).corner_radii(radius));
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::config::presets::{Preset, Shown, TitleShown, ToolsShown};
    use wormhole_core::onboarding::tailor::Question;

    use super::{BAR, Fill, PAD, PANEL_W, Piece, Spec, pieces};

    const W: f32 = 118.0;
    const H: f32 = 78.0;

    fn drawn(shown: &Shown) -> Vec<Piece> {
        pieces(&Spec::of(shown, 4), W, H)
    }

    fn has(pieces: &[Piece], fill: Fill) -> bool {
        pieces.iter().any(|piece| piece.fill == fill)
    }

    #[test]
    fn every_block_is_inside_the_sketch() {
        for preset in Preset::ALL {
            for piece in drawn(&Shown::of(preset)) {
                let at = piece.at;
                assert!(at.x >= 0.0 && at.y >= 0.0, "{preset:?} {piece:?}");
                assert!(at.x + at.w <= W + 0.001 && at.y + at.h <= H + 0.001, "{preset:?} {piece:?}");
            }
        }
    }

    #[test]
    fn the_parts_a_window_shows_are_the_blocks_drawn() {
        let zen = drawn(&Shown::of(Preset::Zen));
        assert!(!has(&zen, Fill::PanelRight) && !has(&zen, Fill::Minimap) && !has(&zen, Fill::Rule));
        let everything = drawn(&Shown::of(Preset::Everything));
        assert!(has(&everything, Fill::PanelRight) && has(&everything, Fill::Minimap));
        assert!(has(&everything, Fill::Rule) && has(&everything, Fill::Chip));
        // The status bar is the bottom strip: the body above it is shorter.
        let status = everything.iter().filter(|piece| piece.fill == Fill::Sidebar).collect::<Vec<_>>();
        assert!(status.iter().any(|piece| piece.at.y + piece.at.h == H), "{status:?}");
    }

    #[test]
    fn the_right_panel_has_a_tab_for_each_tool() {
        let tabs = |tools: ToolsShown| {
            let shown = Shown { panel_open: true, tools, ..Shown::of(Preset::Balanced) };
            // A tab is a square block in the panel's first row.
            drawn(&shown)
                .iter()
                .filter(|piece| piece.at.x >= W - PANEL_W && piece.at.y == PAD && piece.at.w == BAR)
                .count()
        };
        assert_eq!(tabs(ToolsShown::Few), 2);
        assert_eq!(tabs(ToolsShown::All), 4, "the most it draws");
    }

    #[test]
    fn a_title_row_longer_than_the_sketch_is_cut_where_it_ends() {
        let shown =
            Shown { title: TitleShown::Everything, panel_open: true, ..Shown::of(Preset::Everything) };
        for piece in drawn(&shown) {
            assert!(piece.at.x + piece.at.w <= W, "{piece:?}");
        }
    }

    /// The cards of a question must look different, or there is nothing to choose by.
    #[test]
    fn no_two_answers_of_a_question_draw_the_same_sketch() {
        for preset in Preset::ALL {
            let shown = Shown::of(preset);
            for question in Question::ALL {
                let sketches: Vec<Vec<Piece>> =
                    (0..question.answers().len()).map(|ix| drawn(&question.preview(ix, &shown))).collect();
                for (a, first) in sketches.iter().enumerate() {
                    for second in &sketches[a + 1..] {
                        assert_ne!(first, second, "{question:?} from {preset:?}");
                    }
                }
            }
        }
    }
}

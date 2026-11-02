//! The tile row: one row for a thing in a Settings list (a coding agent, a Probe, a project, an update)
//! and the empty state for a list with nothing in it.
//!
//! Every tile row is at least [`TILE_ROW_H`] tall: a tile, a name with an optional solid chip, one line
//! of hint, and controls that end on the column's edge. A row that has a sentence to say about its state
//! (a Probe that changed, a folder that can't be read) carries it as a note under the hint, in the colour
//! of what it says, and grows by that line; anything longer goes under the row as a notice.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::AgentId;

use super::info::setting_info;
use super::scale::{Edges, TextRole};
use super::tile::{TileFace, brand_tile};
use crate::settings::UiColors;

/// A tile row's height: the same for every row of every list.
pub const TILE_ROW_H: f32 = 56.0;
/// The tile's side.
pub const ROW_TILE: f32 = 28.0;
/// The line icon on a tile.
const ROW_TILE_ICON: f32 = 14.0;
/// A chip's height, and the room it keeps between the name and itself.
const CHIP_ROW_H: f32 = 20.0;

/// What a tile row's tile shows.
#[derive(Clone, Copy)]
pub enum RowTile {
    /// A coding agent: its logo on the colour the registry gives it (`agents::Agent::tile`).
    Agent(AgentId),
    /// A line icon in white on a fill: a Probe's or a project's, `settings.nav.tile.*` or the
    /// primary for an update.
    Icon { fill: Hsla, icon: IconName },
}

/// What a [`solid_chip`] says about its row, which picks its fill (`chip.<kind>.fill`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChipKind {
    /// Something is as it should be: On, Trusted.
    Good,
    /// Something wants a look: Changed, Needs repair.
    Warn,
    /// Something is wrong: Can't be enabled.
    Danger,
    /// A plain state: Off, Not trusted, Needs code.
    Quiet,
    /// A label rather than a state: Preview, on the fill of an info notice.
    Info,
}

impl ChipKind {
    fn fill(self, ui: UiColors) -> Hsla {
        match self {
            Self::Good => ui.chip_good,
            Self::Warn => ui.chip_warn,
            Self::Danger => ui.chip_danger,
            Self::Quiet => ui.chip_quiet,
            Self::Info => ui.notice_info,
        }
    }
}

/// A short state word on a solid fill in the kind's colour, white on it: the chip of a [`tile_row`].
#[track_caller]
pub fn solid_chip(ui: UiColors, kind: ChipKind, text: impl Into<SharedString>) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(px(CHIP_ROW_H))
        .px_2()
        .rounded(ui.radius_chip)
        .bg(kind.fill(ui))
        .text_color(ui.chip_text)
        .text_caption()
        .font_weight(FontWeight::SEMIBOLD)
        .whitespace_nowrap()
        .child(text.into())
}

/// The parts of a [`tile_row`].
pub struct TileRowParts {
    pub tile: RowTile,
    pub name: SharedString,
    /// The name's colour when it states something (an update's status); the text colour otherwise.
    pub name_color: Option<Hsla>,
    /// The state, as a solid chip after the name.
    pub chip: Option<(ChipKind, SharedString)>,
    /// One line under the name, muted, cut at the controls.
    pub hint: Option<SharedString>,
    /// One sentence about the row's state under the hint, in the colour of what it says (the row grows by it).
    pub note: Option<(Hsla, SharedString)>,
    /// What a person may read about the row that is not worth a line: behind an (i) after the name.
    pub more: Option<SharedString>,
    /// The controls, which end on the row's right edge.
    pub controls: Vec<AnyElement>,
}

impl TileRowParts {
    pub fn new(tile: RowTile, name: impl Into<SharedString>) -> Self {
        Self {
            tile,
            name: name.into(),
            name_color: None,
            chip: None,
            hint: None,
            note: None,
            more: None,
            controls: Vec::new(),
        }
    }
}

/// One tile row: the tile, the name and chip over the hint, and the controls, [`TILE_ROW_H`] tall with a
/// hairline above like a setting's row. VoiceOver reads the name, then the chip and the hint as its
/// description; the controls are tab stops of their own.
#[track_caller]
pub fn tile_row(id: impl Into<ElementId>, ui: UiColors, parts: TileRowParts) -> Stateful<Div> {
    let TileRowParts { tile, name, name_color, chip, hint, note, more, controls } = parts;
    let id = id.into();
    let grows = note.is_some();
    // One line per fact, read after the name; a row with one fact says it as it is, with no copy.
    let spoken = {
        let mut said =
            [chip.as_ref().map(|(_, chip)| chip), hint.as_ref(), note.as_ref().map(|(_, note)| note)]
                .into_iter()
                .flatten();
        match (said.next(), said.next()) {
            (None, _) => None,
            (Some(only), None) => Some(only.clone()),
            (Some(first), Some(second)) => {
                let mut all = format!("{first}. {second}");
                for more in said {
                    all.push_str(". ");
                    all.push_str(more);
                }
                Some(SharedString::from(all))
            }
        }
    };
    let face = match tile {
        RowTile::Agent(agent) => brand_tile(ui, TileFace::Agent(agent), ROW_TILE),
        RowTile::Icon { fill, icon } => div()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .size(px(ROW_TILE))
            .rounded(ui.radius)
            .bg(fill)
            .child(Icon::new(icon).with_size(px(ROW_TILE_ICON)).text_color(ui.tile_mark_brand))
            .into_any_element(),
    };
    div()
        .id(id.clone())
        .role(Role::Group)
        .aria_label(name.clone())
        .when_some(spoken, |d, spoken| d.aria_description(spoken))
        .flex()
        .items_center()
        .gap_3()
        .w_full()
        // A fixed height costs the frame nothing to work out; only a row with a note has to measure.
        .when(grows, |row| row.min_h(px(TILE_ROW_H)))
        .when(!grows, |row| row.h(px(TILE_ROW_H)))
        .py_2()
        .rule_top(ui)
        // Decoration: the name says what the tile would.
        .child(face)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .min_w_0()
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_body()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(name_color.unwrap_or(ui.text))
                                .child(name),
                        )
                        .children(more.map(|more| setting_info(id.clone(), ui, more, Vec::new())))
                        .children(chip.map(|(kind, text)| solid_chip(ui, kind, text))),
                )
                .children(
                    hint.map(|hint| {
                        div().min_w_0().truncate().text_caption().text_color(ui.muted).child(hint)
                    }),
                )
                .children(
                    note.map(|(color, note)| div().min_w_0().text_caption().text_color(color).child(note)),
                ),
        )
        .child(div().flex_none().flex().items_center().gap_2().children(controls))
}

/// The parts of an [`empty_list`].
pub struct EmptyList {
    pub icon: IconName,
    pub title: SharedString,
    /// What to do about it, in a line or two.
    pub hint: SharedString,
    /// Buttons under the words, centred.
    pub actions: Vec<AnyElement>,
}

/// A list with nothing in it, said once: no agents on this Mac, no Probes, no projects, no terminals to
/// import from. A card with an icon, a title, what to do and the buttons that do it, so the page doesn't
/// read as broken.
#[track_caller]
pub fn empty_list(ui: UiColors, parts: EmptyList) -> Div {
    let EmptyList { icon, title, hint, actions } = parts;
    let actions = (!actions.is_empty())
        .then(|| div().flex().flex_wrap().justify_center().gap_2().pt_2().children(actions));
    super::card(ui, ui.border)
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .px_6()
        .py_4()
        .text_center()
        .child(Icon::new(icon).with_size(px(super::scale::EMPTY_GLYPH)).text_color(ui.muted))
        .child(div().text_body().font_weight(FontWeight::SEMIBOLD).text_color(ui.text).child(title))
        .child(div().text_caption().text_color(ui.muted).child(hint))
        .children(actions)
}

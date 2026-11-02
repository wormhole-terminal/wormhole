//! The tile row: one row for a thing in a Settings list (a coding agent, a Probe, a project, an update)
//! and the empty state for a list with nothing in it.
//!
//! Every tile row is at least 56 tall: a tile, a name with an optional solid chip, one line
//! of hint, and controls that end on the column's edge. A row that has a sentence to say about its state
//! (a Probe that changed, a folder that can't be read) carries it as a note under the hint, in the colour
//! of what it says, and grows by that line; anything longer goes under the row as a notice.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::AgentId;

use super::info::setting_info;
use super::tile::{TileFace, brand_tile};
use crate::settings::UiColors;
use crate::ui::{Look, SolidChipProps, TileRowProps};

/// The tile's side.
pub const ROW_TILE: f32 = 28.0;

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

/// A short state word on a solid fill in the kind's colour, white on it: the chip of a [`tile_row`].
/// Drawn by the `SolidChip` component.
#[derive(IntoElement)]
pub struct SolidChip {
    kind: ChipKind,
    text: SharedString,
}

pub fn solid_chip(_: UiColors, kind: ChipKind, text: impl Into<SharedString>) -> SolidChip {
    SolidChip { kind, text: text.into() }
}

impl RenderOnce for SolidChip {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (good, warn, danger, primary) = (
            self.kind == ChipKind::Good,
            self.kind == ChipKind::Warn,
            self.kind == ChipKind::Danger,
            self.kind == ChipKind::Info,
        );
        SolidChipProps { text: self.text, good, warn, danger, primary }.render(&Look::of(cx))
    }
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

/// One tile row: the tile, the name and chip over the hint, and the controls, drawn by the `TileRow`
/// component. VoiceOver reads the name, then the chip and the hint as its description; the controls are tab
/// stops of their own.
#[track_caller]
pub fn tile_row(id: impl Into<ElementId>, ui: UiColors, parts: TileRowParts, cx: &App) -> Stateful<Div> {
    let TileRowParts { tile, name, name_color, chip, hint, note, more, controls } = parts;
    let id = id.into();
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
    let (tile_fill, brand, icon) = match tile {
        RowTile::Agent(agent) => {
            (transparent_black(), Some(brand_tile(ui, TileFace::Agent(agent), ROW_TILE)), None)
        }
        RowTile::Icon { fill, icon } => (fill, None, Some(icon)),
    };
    let kind = chip.as_ref().map(|(kind, _)| *kind);
    TileRowProps {
        id: id.clone(),
        name,
        spoken,
        name_color: name_color.unwrap_or(ui.text),
        tile_fill,
        brand,
        icon: icon.map(Into::into),
        info: more.map(|more| setting_info(id, ui, more, Vec::new()).into_any_element()),
        chip: chip.map(|(_, text)| text),
        good: kind == Some(ChipKind::Good),
        warn: kind == Some(ChipKind::Warn),
        danger: kind == Some(ChipKind::Danger),
        primary: kind == Some(ChipKind::Info),
        hint,
        note: note.as_ref().map(|(_, note)| note.clone()),
        note_color: note.map_or(ui.muted, |(color, _)| color),
        controls: div().flex().items_center().gap_2().children(controls).into_any_element(),
    }
    .render(&Look::of(cx))
}

/// The parts of an [`empty_list`].
pub struct EmptyList {
    pub icon: IconName,
    pub title: SharedString,
    /// What to do about it, in a line or two.
    pub hint: SharedString,
    /// Buttons at the row's end.
    pub actions: Vec<AnyElement>,
}

/// A list with nothing in it, said once: no agents on this Mac, no Probes, no projects, no terminals to
/// import from. It is a tile row like the ones the list would have, in a card of its own, so the page
/// doesn't read as broken.
#[track_caller]
pub fn empty_list(ui: UiColors, parts: EmptyList, cx: &App) -> Stateful<Div> {
    let EmptyList { icon, title, hint, actions } = parts;
    let fill =
        crate::theme_bridge::token(&cx.global::<crate::settings::SettingsStore>().tokens, "tile.background");
    let mut row = TileRowParts::new(RowTile::Icon { fill, icon }, title);
    row.note = Some((ui.muted, hint));
    row.controls = actions;
    tile_row("empty-list", ui, row, cx)
}

/// An [`empty_list`] in a card of its own, for a page that is not drawing its rows in one.
#[track_caller]
pub fn empty_card(ui: UiColors, parts: EmptyList, cx: &App) -> super::GroupCard {
    super::group_card(ui, [empty_list(ui, parts, cx).into_any_element()])
}

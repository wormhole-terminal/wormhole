//! A row as it looks, before anything answers to it: where it sits in the listing (`Place`), what it
//! shows and what its tooltip and VoiceOver say (`RowFace`). Must not wire anything (click, drag, menu).

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::sidebar::hints::keycap;
use wormhole_core::view::sidebar::wording::place_detail;
use wormhole_core::view::sidebar::{Card, Entry, Listing, SplitMarks};

use super::super::{BODY_REM, CAPTION_REM, two_line_tooltip};
use super::Frame;
use super::label::row_label;
use super::second_line::{SecondLine, second_line};
use super::shift::RowShift;
use super::tile::{tile_parts, tile_shifting};
use crate::sidebar::facts::RowInfo;
use crate::sidebar::motion::draw_bracket;
use crate::style::home_dir;
use crate::style::{self, DetailRoll, SessionRow, TextRole};

/// What a glyph or a keycap takes at the right of a line, gap included, for knowing when the
/// name is cut off.
const TRAIL_W: f32 = 24.0;
const KEYCAP_TRAIL_W: f32 = 36.0;
/// A count at the right of line 1 (notes, ±N), gap included.
const DETAIL_W: f32 = 36.0;
/// The pin after a pinned terminal's name.
const MARK_W: f32 = 20.0;
/// The gap between line 1's parts (`gap_2`).
const LINE_GAP: f32 = 8.0;

/// A code-font character at caption size, roughly, and the room a clock takes.
const MONO_CHAR_W: f32 = 7.2;
const CLOCK_W: f32 = 48.0;
/// How many characters of a path fit on a row's second line before it is cut from the front.
pub(in crate::sidebar) const PLACE_CHARS: usize = 38;

/// Where a split's line runs in a terminal's row: through the middle of its tile, which stands on it.
fn split_line_x(metrics: style::Metrics) -> f32 {
    metrics.row_px + metrics.tile / 2.0
}

/// Where a row sits in the listing.
pub(in crate::sidebar) struct Place {
    /// It is the terminal on screen.
    pub(in crate::sidebar) selected: bool,
    /// The only row of its card: the card takes the selection and hover colours.
    pub solo: bool,
    pub(in crate::sidebar) position: usize,
    pub(in crate::sidebar) of: usize,
    pub(in crate::sidebar) number: Option<usize>,
    /// It is in a group.
    pub(in crate::sidebar) grouped: bool,
    /// What it shows of its split.
    pub(in crate::sidebar) marks: SplitMarks,
    /// The row above it in its card counts the same changes of the same repository, so this one
    /// leaves them out.
    pub(in crate::sidebar) changes_above: bool,
    /// The split's bracket along it is being drawn, in this batch of the list's motion (a split just made).
    pub(in crate::sidebar) bracket_draws: Option<u64>,
    /// Its terminal's status is changing, and how that is played.
    pub(in crate::sidebar) status: Option<RowShift>,
}

impl Place {
    /// Where the row at line `at` of `listing` sits, for the terminal at index `active` being the one on
    /// screen and `cards` the rows being drawn in cards: `None` for a line that is not a row. The one place
    /// that works it out, so the list's rows and the rows leaving it are set alike.
    pub(in crate::sidebar) fn of(
        listing: &Listing,
        infos: &[RowInfo],
        at: usize,
        active: usize,
        cards: bool,
    ) -> Option<Self> {
        let Entry::Row { session, band, position, of, number, marks } = *listing.entries.get(at)? else {
            return None;
        };
        let info = infos.get(session)?;
        // Only inside a group's card, and only for a repository the row above counts the same changes of.
        let above = match at.checked_sub(1).and_then(|prev| listing.entries.get(prev)) {
            Some(&Entry::Row { session: above, band: above_band, .. }) => {
                infos.get(above).map(|i| (above_band, i))
            }
            _ => None,
        };
        let changes_above = band.is_some()
            && info.repo.is_some()
            && info.changes.is_some()
            && above.is_some_and(|(above_band, i)| {
                above_band == band && i.repo == info.repo && i.changes == info.changes
            });
        Some(Self {
            selected: info.ix == active,
            // Only a terminal in no group is a card of its own.
            solo: cards && matches!(listing.card(at), Some(Card::Loose(_))),
            position,
            of,
            number,
            grouped: band.is_some(),
            marks,
            changes_above,
            bracket_draws: None,
            status: None,
        })
    }
}

/// A row as it looks, before anything answers to it: what it shows, what its tooltip says when a name is
/// cut off and what VoiceOver says. A row that is leaving the list is drawn from this alone.
pub(in crate::sidebar) struct RowFace {
    pub(in crate::sidebar) row: Stateful<Div>,
    /// The name and the second line, in full, for a row whose text is cut.
    pub(super) tooltip: Option<(SharedString, SharedString)>,
    /// What VoiceOver says of the row.
    pub(super) label: String,
    /// The hover group the row's × and its faded name share.
    pub(super) group: SharedString,
}

/// `info`'s row at `place`, drawn but not wired: no click, drag, menu or focus (`Sidebar::render_row` adds
/// them). `name_edit` is the field that stands where the name is while the terminal is renamed.
pub(in crate::sidebar) fn row_face(
    info: &RowInfo,
    place: &Place,
    frame: &Frame,
    name_edit: Option<AnyElement>,
    window: &Window,
    cx: &App,
) -> RowFace {
    let ui = frame.ui;
    let id = info.id;
    let selected = place.selected;
    let group = SharedString::from("terminal-row");

    // Every row leads with its status tile: what the terminal is inside, what it is doing round
    // it. It says the bell too, so nothing is left for the right.
    let shift = place.status.as_ref().map(|played| played.shift);
    let tile = tile_shifting(info, frame, selected, frame.metrics.tile, shift, cx);
    // Row details: the notes and tasks still open, the row's own changed files.
    let quiet = if selected { ui.selected_muted } else { ui.muted };
    let notes = info.extra.filter(|_| frame.show_notes).map(|extra| {
        div()
            .flex()
            .items_center()
            .gap_1()
            .text_caption()
            .tabular_nums()
            .text_color(quiet)
            .child(Icon::new(IconName::StickyNote).xsmall())
            .child(extra.count.to_string())
    });
    let changes = info
        .changes
        .filter(|_| frame.show_changes && !place.changes_above)
        .map(|n| div().text_caption().tabular_nums().text_color(quiet).child(format!("±{n}")));
    // What a split says of this terminal, at the right of its first line.
    let chain = place.marks.chain.then(|| style::chain_mark(("chain", id as u64), ui));
    let trail = (notes.is_some() || changes.is_some() || chain.is_some()).then(|| {
        div()
            .flex()
            .items_center()
            .gap_2()
            // The close × takes this place under the pointer.
            .group_hover(group.clone(), |s| s.invisible())
            .children(chain)
            .children(notes)
            .children(changes)
            .into_any_element()
    });

    // The second line: every regular row has one. What the terminal
    // has to report comes first; a quiet one says where it is, and where even that is
    // unknown it says what it is, so no row is a single line.
    let SecondLine { text: detail, tone, mono, trail: detail_trail } = line_of(info, frame);
    // Line rolls: the words it had roll away as the new ones come.
    let detail_roll = place.status.as_ref().and_then(|played| {
        let was = played.was.as_ref()?;
        Some(DetailRoll {
            from: SharedString::from(was.text.clone()?),
            from_tone: was.tone,
            from_font: was.mono.then(|| frame.code_font.clone()),
            shift: played.shift,
            to: tile_parts(info, frame).1,
        })
    });

    let hint = place.number.filter(|_| frame.hints).map(|n| SharedString::from(keycap(n)));
    // After the name: the pin, and the worktree mark the band carries too, with the
    // same tooltip, for the flat list and a terminal that isn't in its band's folder.
    let mark_color = if selected { ui.selected_muted } else { ui.faint };
    let worktree_mark = info.worktree.as_ref().map(|main| {
        let name =
            main.file_name().map_or_else(|| main.display().to_string(), |n| n.to_string_lossy().into_owned());
        let first = SharedString::from(format!("{} · worktree of {name}", info.name));
        let second = SharedString::from(
            info.cwd
                .as_deref()
                .map(|f| wormhole_core::view::sidebar::display_path(f, home_dir()))
                .unwrap_or_default(),
        );
        div()
            .id("row-worktree-mark")
            .flex()
            .items_center()
            .text_color(mark_color)
            .child(Icon::new(IconName::GitFork).xsmall())
            .tooltip(move |window, cx| two_line_tooltip(first.clone(), second.clone(), ui, window, cx))
    });
    let marks = usize::from(info.pinned) + usize::from(worktree_mark.is_some());
    let mark = (marks > 0).then(|| {
        div()
            .flex()
            .items_center()
            .gap_1()
            .children(info.pinned.then(|| Icon::new(IconName::Pin).xsmall().text_color(mark_color)))
            .children(worktree_mark)
    });

    // Tooltip only when something is cut off: the name, or the second line.
    // Only what is drawn: the ±N is left out when the row above counts the same changes.
    let details = (frame.show_notes && info.extra.is_some()) as u8
        + (frame.show_changes && !place.changes_above && info.changes.is_some()) as u8;
    let right = if hint.is_some() { KEYCAP_TRAIL_W } else { 0.0 }
        + f32::from(details) * DETAIL_W
        + if place.marks.chain { TRAIL_W } else { 0.0 };
    // What the row's sides and its lead take: the card's margin and padding, the row's own
    // padding, and the tile or the lead glyph with its gap.
    let side = frame.metrics.card_margin + frame.metrics.card_pad + frame.metrics.row_px;
    let lead_w = frame.metrics.tile + frame.metrics.tile_gap;
    // With nothing at the line's end the gap before its spacer still follows the name (the
    // details' widths count theirs).
    let name_room = frame.width
        - 2.0 * side
        - lead_w
        - right
        - marks as f32 * MARK_W
        - if right == 0.0 { LINE_GAP } else { 0.0 };
    let name_weight = if selected || info.unread { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM };
    let cut_name = frame.measure(&info.name, BODY_REM, name_weight, window) > name_room;
    let detail_room = frame.width - 2.0 * side - lead_w - detail_trail.as_ref().map_or(0.0, |_| CLOCK_W);
    let cut_detail = !mono
        && detail
            .as_deref()
            .is_some_and(|d| frame.measure(d, CAPTION_REM, FontWeight::MEDIUM, window) > detail_room)
        || mono && detail.as_deref().is_some_and(|d| d.chars().count() as f32 * MONO_CHAR_W > detail_room);
    let tooltip = (cut_name || cut_detail).then(|| {
        let where_ =
            info.cwd.as_deref().map(|folder| wormhole_core::view::sidebar::display_path(folder, home_dir()));
        let second = [where_, detail.clone()].into_iter().flatten().collect::<Vec<_>>().join(" · ");
        (info.name.clone(), SharedString::from(second))
    });

    let label = row_label(info, selected, place);

    let row = style::session_row(
        ("session", id as u64),
        ui,
        SessionRow {
            glyph: div().into_any_element(),
            name_edit,
            name: info.name.clone(),
            detail: detail.map(SharedString::from),
            tone,
            selected,
            hint,
            emphasis: info.unread,
            detail_font: mono.then(|| frame.code_font.clone()),
            mark: mark.map(IntoElement::into_any_element),
            trail,
            detail_trail: detail_trail.map(SharedString::from),
            tile: Some(tile),
            behind: place.marks.bracket.map(|part| {
                let line = style::split_line(part, split_line_x(frame.metrics), ui);
                match place.bracket_draws {
                    Some(batch) => draw_bracket(line, id, batch, part),
                    None => line.into_any_element(),
                }
            }),
            card: true,
            solo: place.solo,
            metrics: frame.metrics,
            // The whole line at rest; under the pointer, when nothing else sits at the line's end
            // to give way to the ×, faded out before it.
            name_fade: frame.fade(&info.name, BODY_REM, name_weight, name_room, window),
            detail_roll,
            name_hover: (right == 0.0)
                .then(|| {
                    frame.fade(&info.name, BODY_REM, name_weight, name_room - style::CLOSE_GLYPH_ROOM, window)
                })
                .flatten()
                .map(|fade| (fade, group.clone())),
        },
    );
    // The card washed in the new status's colour, which fades away (the Card wash variant).
    let wash = place.status.as_ref().and_then(|played| {
        let (_, to) = tile_parts(info, frame);
        played.shift.wash(("status-wash", id as u64), ui, to, frame.metrics.row_radius)
    });
    let row = row.children(wash);
    RowFace { row, tooltip, label, group }
}

/// What a terminal's second line says: the words, their tone, whether they are a command and what sits at
/// its right.
pub(in crate::sidebar) fn line_of(info: &RowInfo, frame: &Frame) -> SecondLine {
    let whereabouts = || place_detail(info.cwd.as_deref(), home_dir(), PLACE_CHARS);
    second_line(info, !frame.metrics.two_lines, whereabouts)
}

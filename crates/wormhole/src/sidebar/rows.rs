//! One terminal's row: the lead glyph or agent logo, the name
//! at x 40, the second line (what the agent does, the command in front, where it is), what
//! sits at x 334 (⌘N keycap, status glyph on the right, bell, close ×), and the states laid
//! over it (its context menu's focus ring, a drag, the flash after a move). A terminal in a
//! split also shows it: the chain mark at the right of its first line when its mates are elsewhere
//! (rows of one split in no group share a card; VoiceOver is told they are on screen).
use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::sidebar::RowState;
use wormhole_core::view::sidebar::SplitMarks;
use wormhole_core::view::sidebar::hints::{keycap, keyshortcut};
use wormhole_core::view::sidebar::spoken::{self, RowSpeech, SplitSpeech};
use wormhole_core::view::sidebar::wording::{agent_detail, ago, clock, place_detail, took};

use super::facts::RowInfo;
use super::input::SidebarInput;
use super::{BODY_REM, CAPTION_REM, Sidebar, SidebarEvent, two_line_tooltip};
use crate::settings::{UiColors, settings_in, ui_in};
use crate::sidebar::SidebarStop;
use crate::sidebar::terminal_drag::DragOrigin;
use crate::style::home_dir;
use crate::style::{self, TextRole};
use crate::style::{Metrics, RowTone, SessionRow, StatusGlyph, StatusTile, TileFace};

/// How long the row that moved to another band stays highlighted, fading out.
const MOVE_FLASH: Duration = Duration::from_millis(600);
/// The close ×: drawn 22 square, this far under the row's top.
const CLOSE_X_TOP: f32 = 3.0;
/// How far before its line's end a name too long for it fades out to nothing, in place of an
/// ellipsis: the name uses the whole width, and the × under the pointer lies over the faded end.
const NAME_FADE: f32 = 32.0;

/// How far the × stands in from the row's right side: its glyph ends where line 1 ends (the row's
/// own padding, and a lone card's), on the right edge of what sits there at rest.
fn close_right(metrics: &style::Metrics, solo: bool) -> f32 {
    let line_end = metrics.row_px + if solo { metrics.card_pad } else { 0.0 };
    line_end - (style::CONTROL_H_SM - style::GLYPH) / 2.0
}
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
/// How much room a name must have to spare to be drawn whole rather than faded.
const FIT_SLACK: f32 = 16.0;

/// What every row of one frame shares: the settings and how text is measured.
pub(super) struct Frame {
    pub(super) ui: UiColors,
    /// What rows, cards and heads measure at the person's density.
    pub(super) metrics: Metrics,
    show_dot: bool,
    pub(super) show_changes: bool,
    show_notes: bool,
    /// The ⌘N keycaps are showing.
    pub(super) hints: bool,
    pub(super) code_font: SharedString,
    pub(super) width: f32,
    text_style: TextStyle,
    rem: Pixels,
}

impl Frame {
    pub(super) fn new(input: &SidebarInput, width: f32, hints: bool, window: &mut Window, cx: &App) -> Self {
        let s = settings_in(&input.profile, cx);
        Self {
            ui: ui_in(&input.profile, cx),
            metrics: Metrics::of(s.density, s.show_groups_as_cards)
                .fading(s.fade_group_colors)
                .with_group_folders(s.show_group_folders),
            show_dot: s.sidebar_status_dot,
            show_changes: s.sidebar_changes,
            show_notes: s.sidebar_notes,
            hints,
            code_font: crate::fonts::code_font_in(&input.profile, cx).0,
            width,
            text_style: window.text_style(),
            rem: window.rem_size(),
        }
    }

    /// How wide `text` is in the interface font at `size` (rems) and `weight`.
    pub(super) fn measure(&self, text: &str, size: f32, weight: FontWeight, window: &Window) -> f32 {
        if text.is_empty() || text.contains('\n') {
            return 0.0;
        }
        let mut run = self.text_style.to_run(text.len());
        run.font.weight = weight;
        let size = rems(size).to_pixels(self.rem);
        f32::from(
            window.text_system().shape_line(SharedString::from(text.to_string()), size, &[run], None).width(),
        )
    }

    /// For a line of text too long for `room`: the bytes that fade out over its last [`NAME_FADE`]
    /// points, and the room it is cut to there. `None` when it fits.
    pub(super) fn fade(
        &self,
        text: &str,
        size: f32,
        weight: FontWeight,
        room: f32,
        window: &Window,
    ) -> Option<style::NameFade> {
        if text.is_empty() || text.contains('\n') || room <= NAME_FADE {
            return None;
        }
        let mut run = self.text_style.to_run(text.len());
        run.font.weight = weight;
        let size = rems(size).to_pixels(self.rem);
        let line = window.text_system().shape_line(SharedString::from(text.to_string()), size, &[run], None);
        // The room is worked out before layout, a few points off at times: a name that only just fits
        // fades too, so it never falls back to an ellipsis.
        if f32::from(line.width()) <= room - FIT_SLACK {
            return None;
        }
        let start = line.closest_index_for_x(px(room - NAME_FADE));
        let end = line.closest_index_for_x(px(room)).max(start);
        Some(style::NameFade { bytes: start..end, room })
    }
}

/// Where a row sits in the listing.
pub(super) struct Place {
    /// The only row of its card: the card takes the selection and hover colours.
    pub solo: bool,
    pub(super) position: usize,
    pub(super) of: usize,
    pub(super) number: Option<usize>,
    /// It is in a group.
    pub(super) grouped: bool,
    /// What it shows of its split.
    pub(super) marks: SplitMarks,
    /// The row above it in its card counts the same changes of the same repository, so this one
    /// leaves them out.
    pub(super) changes_above: bool,
}

/// A terminal's status tile: its agent's logo (when the rows show it) or the terminal symbol, and
/// its state round it; the bell is a badge on a terminal that has nothing louder to say.
pub(super) fn tile_for(info: &RowInfo, frame: &Frame, selected: bool, size: f32, cx: &App) -> AnyElement {
    let (face, state) = tile_parts(info, frame);
    let id = ElementId::Name(SharedString::from(format!("tile-{}-{}", info.id, size as u32)));
    style::status_tile_at(id, frame.ui, StatusTile { face, glyph: state, selected, mini: false }, size, cx)
}

/// What a terminal's tile shows: the face and the state, for a tile drawn later (the tab bar's
/// switcher builds its content without the workspace).
pub(super) fn tile_parts(info: &RowInfo, frame: &Frame) -> (TileFace, StatusGlyph) {
    let face = match info.agent {
        Some(agent) => TileFace::Agent(agent),
        None => TileFace::Terminal,
    };
    let state = match glyph_for(info) {
        _ if !frame.show_dot => StatusGlyph::Idle,
        StatusGlyph::Shell | StatusGlyph::Idle | StatusGlyph::Terminal { unread: false } if info.bell => {
            StatusGlyph::Bell
        }
        state => state,
    };
    (face, state)
}

/// The glyph a terminal's state is drawn with.
pub(super) fn glyph_for(info: &RowInfo) -> StatusGlyph {
    match (info.told(), info.state) {
        (_, RowState::Exited) => StatusGlyph::Shell,
        (true, RowState::Working) => StatusGlyph::Working,
        (true, RowState::NeedsYou) => StatusGlyph::NeedsYou,
        (true, RowState::Finished) => StatusGlyph::Done,
        (_, RowState::Failed) => StatusGlyph::Failed,
        (true, RowState::Idle) => StatusGlyph::Idle,
        (false, _) if info.command.is_some() => StatusGlyph::Terminal { unread: false },
        // The last command failed and nothing runs now: the row keeps saying so until the next one
        // (seen or not), in place of the quiet terminal and its output.
        (false, _) if info.failure.is_some() => StatusGlyph::Failed,
        (false, _) if info.unread => StatusGlyph::Terminal { unread: true },
        (false, _) => StatusGlyph::Shell,
    }
}

/// A row's second line: its words, their color, whether they are a command (code font; a path is
/// in the interface font, as the title row and a pane's header show it) and what sits at the
/// line's right.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct SecondLine {
    pub(super) text: Option<String>,
    pub(super) tone: RowTone,
    pub(super) mono: bool,
    pub(super) trail: Option<String>,
}

/// The second line of every regular row: what the terminal has to report comes first; a quiet
/// one says where it is (`whereabouts`), and where even that is unknown it says what it is, so no
/// row is a single line. A compact or dense row has none (`compact`).
pub(super) fn second_line(
    info: &RowInfo,
    compact: bool,
    whereabouts: impl FnOnce() -> Option<String>,
) -> SecondLine {
    let line = |text: Option<String>, tone, mono, trail| SecondLine { text, tone, mono, trail };
    if compact {
        return line(None, RowTone::Plain, false, None);
    }
    if info.told() {
        // An agent's, or a program's own report: its state in words.
        return match agent_detail(info.state, info.message.as_deref(), info.since()) {
            Some(text) => line(Some(text), tone_of(info.state), false, None),
            None => match whereabouts() {
                Some(place) => line(Some(place), RowTone::Plain, false, None),
                None => line(Some("Idle".to_string()), RowTone::Plain, false, None),
            },
        };
    }
    let trail = if info.unread { Some("new output".to_string()) } else { info.running().map(clock) };
    match (&info.command, &info.failure) {
        (Some(command), _) => line(Some(command.clone()), RowTone::Plain, true, trail),
        // The last command failed: what it was and how it ended, in danger, and when, until the
        // next command starts.
        (None, Some(failure)) => {
            let exit = format!("exit {}", failure.code);
            let text = match &failure.command {
                Some(command) => format!("{command} · {exit}"),
                None => exit,
            };
            line(Some(text), RowTone::Failed, true, Some(ago(failure.since())))
        }
        (None, None) => match whereabouts() {
            Some(place) => line(Some(place), RowTone::Plain, false, None),
            None => line(Some("Shell".to_string()), RowTone::Plain, false, None),
        },
    }
}

/// The part of a row's text that moves with time ("Working · 3m", a command's "1:12"), for
/// knowing when a tick has to redraw.
pub(super) fn time_text(info: &RowInfo, compact: bool) -> Option<String> {
    if compact {
        return None;
    }
    match (info.told(), info.state) {
        (true, RowState::Working) if info.message.is_none() => took(info.since()),
        (true, RowState::Finished | RowState::Failed) if info.message.is_none() => Some(ago(info.since())),
        (false, _) => info.running().map(clock).or_else(|| info.failure.as_ref().map(|f| ago(f.since()))),
        _ => None,
    }
}

impl Sidebar {
    pub(super) fn render_row(
        &self,
        info: &RowInfo,
        place: &Place,
        frame: &Frame,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = frame.ui;
        let id = info.id;
        let selected = info.ix == self.input.active;
        let group = SharedString::from("terminal-row");

        // Every row leads with its status tile: what the terminal is inside, what it is doing round
        // it. It says the bell too, so nothing is left for the right.
        let tile = tile_for(info, frame, selected, frame.metrics.tile, cx);
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
        let whereabouts = || place_detail(info.cwd.as_deref(), home_dir(), PLACE_CHARS);
        let SecondLine { text: detail, tone, mono, trail: detail_trail } =
            second_line(info, !frame.metrics.two_lines, whereabouts);

        let hint = place.number.filter(|_| frame.hints).map(|n| SharedString::from(keycap(n)));
        // After the name: the pin, and the worktree mark the band carries too, with the
        // same tooltip, for the flat list and a terminal that isn't in its band's folder.
        let mark_color = if selected { ui.selected_muted } else { ui.faint };
        let worktree_mark = info.worktree.as_ref().map(|main| {
            let name = main
                .file_name()
                .map_or_else(|| main.display().to_string(), |n| n.to_string_lossy().into_owned());
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
            || mono
                && detail.as_deref().is_some_and(|d| d.chars().count() as f32 * MONO_CHAR_W > detail_room);
        let tooltip = (cut_name || cut_detail).then(|| {
            let where_ = info
                .cwd
                .as_deref()
                .map(|folder| wormhole_core::view::sidebar::display_path(folder, home_dir()));
            let second = [where_, detail.clone()].into_iter().flatten().collect::<Vec<_>>().join(" · ");
            (info.name.clone(), SharedString::from(second))
        });

        let label = row_label(info, selected, place);

        let row = style::session_row(
            ("session", id as u64),
            ui,
            SessionRow {
                glyph: div().into_any_element(),
                name_edit: self.rename_field(id, cx),
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
                behind: place
                    .marks
                    .bracket
                    .map(|part| style::split_line(part, split_line_x(frame.metrics), ui).into_any_element()),
                card: true,
                solo: place.solo,
                metrics: frame.metrics,
                // The whole line at rest; under the pointer, when nothing else sits at the line's end
                // to give way to the ×, faded out before it.
                name_fade: frame.fade(&info.name, BODY_REM, name_weight, name_room, window),
                name_hover: (right == 0.0)
                    .then(|| {
                        frame.fade(
                            &info.name,
                            BODY_REM,
                            name_weight,
                            name_room - style::CLOSE_GLYPH_ROOM,
                            window,
                        )
                    })
                    .flatten()
                    .map(|fade| (fade, group.clone())),
            },
        );
        let flash = self.flash.filter(|(flashed, _)| *flashed == id).map(|(_, generation)| {
            div().absolute().top_0().left_0().size_full().bg(ui.highlight).with_animation(
                ("move-flash", generation),
                Animation::new(MOVE_FLASH),
                |d, t| d.opacity(1.0 - t),
            )
        });
        let dragged = cx.has_active_drag()
            && self.input.terminal_drag.is_some_and(|(drag, origin)| drag == id && origin == DragOrigin::Row);
        // The keys' place: VoiceOver follows it, and the ring shows while keys move it.
        let (keyed, ring) = self.keyed(&SidebarStop::Row(id), window);
        let row = row
            .group(group.clone())
            .relative()
            .role(Role::TreeItem)
            .aria_label(label)
            .aria_selected(selected)
            .aria_level(if place.grouped { 2 } else { 1 })
            .aria_position_in_set(place.position)
            .aria_size_of_set(place.of)
            .when_some(place.number, |d, n| d.aria_keyshortcuts(keyshortcut(n)))
            .when(keyed, |d| d.aria_active_descendant())
            .when(dragged, |d| d.opacity(style::DRAG_LIFT_OPACITY))
            .on_click(
                cx.listener(move |_, _, _, cx| {
                    cx.emit(SidebarEvent::Activate { id, then_focus_list: false })
                }),
            )
            .when_some(tooltip, |d, (name, second)| {
                d.tooltip(move |window, cx| two_line_tooltip(name.clone(), second.clone(), ui, window, cx))
            })
            .children(flash)
            .when(self.menu_row == Some(id) || ring, |d| {
                d.child(style::focus_ring(ui, style::RingPlacement::Inside, px(style::CARD_ROW_RADIUS)))
            })
            .when(!frame.hints, |d| {
                d.child(self.close_button(
                    info,
                    selected,
                    &group,
                    close_right(&frame.metrics, place.solo),
                    ui,
                    cx,
                ))
            })
            .map(|d| self.with_drag(d, info, cx));
        // Last: the menu wraps the row.
        self.with_row_menu(row, info, cx).into_any_element()
    }

    /// The × that closes a terminal: on the row under the pointer, the selected one too.
    pub(super) fn close_button(
        &self,
        info: &RowInfo,
        selected: bool,
        group: &SharedString,
        right: f32,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = info.id;
        let fill = if selected { style::SelectedFill::Tint } else { style::SelectedFill::Plain };
        // No tooltip while a close question is up: the pointer still rests on the ×, and GPUI
        // would show (or keep) its tooltip beside the dialog. Dropping it clears what is pending.
        let asking = self.input.asking;
        div()
            .absolute()
            .top(px(CLOSE_X_TOP))
            .right(px(right))
            .size(px(style::CONTROL_H_SM))
            .invisible()
            .group_hover(group.clone(), |s| s.visible())
            .child(style::hit_area(
                ("close-hit", info.id as u64),
                style::CONTROL_H_SM,
                style::close_x(("close", info.id as u64), ui, fill),
                |hit| {
                    hit.role(Role::Button)
                        .aria_label("Close terminal")
                        .when(!asking, |d| {
                            d.tooltip(|window, cx| Tooltip::new("Close terminal").build(window, cx))
                        })
                        .on_click(cx.listener(move |_, _, _, cx| {
                            cx.stop_propagation();
                            cx.emit(SidebarEvent::Close(id));
                        }))
                },
            ))
            .into_any_element()
    }
}

/// What VoiceOver says of a row: its name, what it is doing, its split and its place in the list.
fn row_label(info: &RowInfo, selected: bool, place: &Place) -> String {
    spoken::row_label_in_split(
        &info.name,
        if info.told() {
            RowSpeech::Agent {
                state: info.state,
                detail: info.message.as_deref().filter(|_| info.state != RowState::Idle),
            }
        } else {
            RowSpeech::Plain {
                command: info.command.as_deref(),
                running: info.running(),
                bell: info.bell,
                unread: info.unread,
                failed: info.failure.as_ref().map(|failure| failure.code),
            }
        },
        spoken::Place { selected, position: place.position, of: place.of, number: place.number },
        SplitSpeech::of(place.marks),
    )
}

/// The second line's color for an agent's state.
fn tone_of(state: RowState) -> RowTone {
    match state {
        RowState::NeedsYou => RowTone::Attention,
        RowState::Finished => RowTone::Done,
        RowState::Failed => RowTone::Failed,
        RowState::Working | RowState::Idle | RowState::Exited => RowTone::Plain,
    }
}

/// A code-font character at caption size, roughly, and the room a clock takes.
const MONO_CHAR_W: f32 = 7.2;
const CLOCK_W: f32 = 48.0;
/// How many characters of a path fit on a row's second line before it is cut from the front.
pub(super) const PLACE_CHARS: usize = 38;

/// Where a split's line runs in a terminal's row: through the middle of its tile, which stands on it.
fn split_line_x(metrics: style::Metrics) -> f32 {
    metrics.row_px + metrics.tile / 2.0
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use wormhole_core::agents::AgentId;
    use wormhole_core::view::sidebar::RowState;

    use wormhole_core::notify::Source;

    use super::super::facts::{RowFailure, ago};
    use super::{RowInfo, SecondLine, glyph_for, second_line, time_text};
    use crate::style::{RowTone, StatusGlyph};

    fn info(agent: bool, state: RowState) -> RowInfo {
        RowInfo {
            agent: agent.then_some(AgentId::Claude),
            source: if agent { Source::Agent } else { Source::Plain },
            state,
            ..RowInfo::test_row(1)
        }
    }

    /// A program that reports its own status, no agent: the row of an agent without its logo.
    fn program(state: RowState) -> RowInfo {
        RowInfo { source: Source::Program, ..info(false, state) }
    }

    fn failed(code: i32, command: Option<&str>, secs: u64) -> RowFailure {
        RowFailure { code, command: command.map(str::to_string), at: ago(Duration::from_secs(secs)) }
    }

    #[test]
    fn agents_show_their_state_and_plain_terminals_the_terminal() {
        assert_eq!(glyph_for(&info(true, RowState::Working)), StatusGlyph::Working);
        assert_eq!(glyph_for(&info(true, RowState::NeedsYou)), StatusGlyph::NeedsYou);
        assert_eq!(glyph_for(&info(true, RowState::Finished)), StatusGlyph::Done);
        assert_eq!(glyph_for(&info(true, RowState::Idle)), StatusGlyph::Idle);
        let bare = info(false, RowState::Idle);
        assert_eq!(glyph_for(&bare), StatusGlyph::Shell, "a bare shell is faint");
        let running = RowInfo { command: Some("cargo test".into()), ..info(false, RowState::Working) };
        assert_eq!(glyph_for(&running), StatusGlyph::Terminal { unread: false });
        let unread = RowInfo { unread: true, ..info(false, RowState::Idle) };
        assert_eq!(glyph_for(&unread), StatusGlyph::Terminal { unread: true });
    }

    #[test]
    fn a_failure_is_a_danger_glyph_for_everyone_until_the_next_command() {
        assert_eq!(glyph_for(&info(true, RowState::Failed)), StatusGlyph::Failed);
        assert_eq!(glyph_for(&program(RowState::Failed)), StatusGlyph::Failed);
        assert_eq!(glyph_for(&info(false, RowState::Failed)), StatusGlyph::Failed);
        // Seen, the status is idle but the row still says the last command failed.
        let seen =
            RowInfo { failure: Some(failed(101, Some("cargo build"), 120)), ..info(false, RowState::Idle) };
        assert_eq!(glyph_for(&seen), StatusGlyph::Failed);
        // A new command is running: the terminal glyph; the failure is history.
        let again = RowInfo { command: Some("cargo build".into()), ..info(false, RowState::Working) };
        assert_eq!(glyph_for(&again), StatusGlyph::Terminal { unread: false });
    }

    #[test]
    fn a_program_that_reports_gets_an_agents_glyphs_and_words() {
        assert_eq!(glyph_for(&program(RowState::Working)), StatusGlyph::Working);
        assert_eq!(glyph_for(&program(RowState::NeedsYou)), StatusGlyph::NeedsYou);
        assert_eq!(glyph_for(&program(RowState::Finished)), StatusGlyph::Done);
        let asks = RowInfo { message: Some("Password:".into()), ..program(RowState::NeedsYou) };
        assert_eq!(
            second_line(&asks, false, || None),
            SecondLine {
                text: Some("Needs input · Password:".into()),
                tone: RowTone::Attention,
                mono: false,
                trail: None
            }
        );
        let done = RowInfo { message: Some("Upgraded 12 packages".into()), ..program(RowState::Finished) };
        assert_eq!(
            second_line(&done, false, || None).text.as_deref(),
            Some("Finished · Upgraded 12 packages")
        );
        assert_eq!(time_text(&done, false), None, "its words don't move");
        let broke = RowInfo { message: Some("rsync failed".into()), ..program(RowState::Failed) };
        let line = second_line(&broke, false, || None);
        assert_eq!((line.text.as_deref(), line.tone), (Some("Failed · rsync failed"), RowTone::Failed));
        let mute = program(RowState::Failed);
        assert_eq!(second_line(&mute, false, || None).text.as_deref(), Some("Failed · 3m ago"));
        assert_eq!(time_text(&mute, false).as_deref(), Some("3m ago"));
        let busy = RowInfo { message: Some("Installing · 40%".into()), ..program(RowState::Working) };
        assert_eq!(second_line(&busy, false, || None).text.as_deref(), Some("Installing · 40%"));
    }

    /// F-08: danger glyph, "command · exit N" in danger, and when it ended.
    #[test]
    fn a_failed_command_says_what_ended_how_and_when() {
        let build =
            RowInfo { failure: Some(failed(101, Some("cargo build"), 120)), ..info(false, RowState::Failed) };
        assert_eq!(
            second_line(&build, false, || Some("~/scratch".into())),
            SecondLine {
                text: Some("cargo build · exit 101".into()),
                tone: RowTone::Failed,
                mono: true,
                trail: Some("2m ago".into()),
            }
        );
        assert_eq!(time_text(&build, false).as_deref(), Some("2m ago"));
        // Seen: the cue stays, though the state is idle now.
        let seen = RowInfo { state: RowState::Idle, ..build };
        assert_eq!(second_line(&seen, false, || None).tone, RowTone::Failed);
        // The shell gave no command: the exit code alone.
        let bare = RowInfo { failure: Some(failed(2, None, 5)), ..info(false, RowState::Failed) };
        assert_eq!(second_line(&bare, false, || None).text.as_deref(), Some("exit 2"));
        // The next command starts: the cue is gone, and the row says what runs.
        let next = RowInfo { command: Some("cargo test".into()), ..info(false, RowState::Working) };
        let line = second_line(&next, false, || None);
        assert_eq!((line.text.as_deref(), line.tone), (Some("cargo test"), RowTone::Plain));
        // Compact rows have no second line.
        assert_eq!(second_line(&bare, true, || None).text, None);
    }

    #[test]
    fn only_moving_times_count_for_redraws() {
        assert_eq!(time_text(&info(true, RowState::Working), false).as_deref(), Some("3m"));
        let told = RowInfo { message: Some("Editing".into()), ..info(true, RowState::Working) };
        assert_eq!(time_text(&told, false), None, "the agent's own words don't move");
        assert_eq!(time_text(&info(true, RowState::Finished), false).as_deref(), Some("3m ago"));
        let running = RowInfo {
            running_since: Some(SystemTime::now() - Duration::from_secs(72)),
            ..info(false, RowState::Working)
        };
        assert_eq!(time_text(&running, false).as_deref(), Some("1:12"));
        assert_eq!(time_text(&running, true), None, "compact rows have no second line");
    }

    fn place(position: usize, of: usize, number: Option<usize>) -> super::Place {
        super::Place {
            solo: false,
            position,
            of,
            number,
            grouped: false,
            marks: wormhole_core::view::sidebar::SplitMarks::default(),
            changes_above: false,
        }
    }

    #[test]
    fn a_rows_label_says_its_name_what_it_does_and_where_it_is() {
        use super::row_label;
        let working = RowInfo { name: "claude".into(), ..info(true, RowState::Working) };
        assert_eq!(
            row_label(&working, true, &place(1, 3, Some(1))),
            "claude, working, selected, 1 of 3, Command 1"
        );
        let asking = RowInfo {
            name: "api".into(),
            message: Some("Allow edit?".into()),
            ..info(true, RowState::NeedsYou)
        };
        assert_eq!(row_label(&asking, false, &place(2, 3, None)), "api, needs input, Allow edit?, 2 of 3");
        let build = RowInfo {
            name: "web".into(),
            command: Some("cargo build --release".into()),
            running_since: Some(SystemTime::now() - Duration::from_secs(72)),
            ..info(false, RowState::Working)
        };
        assert!(row_label(&build, false, &place(3, 3, None)).starts_with("web, running cargo"));
        let failed =
            RowInfo { failure: Some(failed(101, Some("cargo test"), 5)), ..info(false, RowState::Failed) };
        assert!(row_label(&failed, false, &place(1, 1, None)).contains("last command failed, exit 101"));
        let idle = info(false, RowState::Idle);
        assert!(row_label(&idle, false, &place(1, 1, None)).contains("shell, nothing running"));
    }
}

//! One terminal's row: the lead glyph or agent logo, the name
//! at x 40, the second line (what the agent does, the command in front, where it is), what
//! sits at x 334 (⌘N keycap, status glyph on the right, bell, close ×), and the states laid
//! over it (its context menu's focus ring, a drag, the flash after a move). A terminal in a
//! split also shows it: the bracket along the gutter, the chain mark and "on screen" at the
//! right of its first line.
use std::time::Duration;

use gpui_kit::component::tooltip::Tooltip;
use wormhole_core::agents;
use wormhole_core::sidebar::SplitMarks;
use wormhole_core::sidebar::hints::{keycap, keyshortcut};
use wormhole_core::sidebar::spoken::{self, RowSpeech, SplitSpeech};
use wormhole_core::sidebar::text::{agent_detail, ago, clock, place_detail, took};

use super::super::focus::SidebarStop;
use super::super::terminal_drag::DragOrigin;
use super::*;
use crate::style::{RowTone, SessionRow, StatusGlyph};

/// How long the row that moved to another band stays highlighted, fading out.
const MOVE_FLASH: Duration = Duration::from_millis(600);
/// The close ×: drawn 22 square, its glyph ending at x 334.
const CLOSE_X_TOP: f32 = 3.0;
const CLOSE_X_RIGHT: f32 = 12.0;
/// What a glyph or a keycap takes at the right of a line, gap included, for knowing when the
/// name is cut off.
const TRAIL_W: f32 = 24.0;
const KEYCAP_TRAIL_W: f32 = 36.0;
/// A count at the right of line 1 (notes, ±N), gap included.
const DETAIL_W: f32 = 36.0;
/// The pin after a pinned terminal's name.
const MARK_W: f32 = 20.0;
/// "● on screen" at the right of a split sibling's first line, gap included.
const ON_SCREEN_W: f32 = 76.0;

/// What every row of one frame shares: the settings and how text is measured.
pub(super) struct Frame {
    pub(super) ui: UiColors,
    pub(super) compact: bool,
    show_dot: bool,
    dot_right: bool,
    show_agent: bool,
    pub(super) show_branch: bool,
    pub(super) show_changes: bool,
    show_notes: bool,
    /// The ⌘N keycaps are showing.
    pub(super) hints: bool,
    code_font: SharedString,
    pub(super) width: f32,
    text_style: TextStyle,
    rem: Pixels,
}

impl Frame {
    pub(super) fn new(ws: &Workspace, width: f32, window: &mut Window, cx: &App) -> Self {
        let s = settings(cx);
        Self {
            ui: ui(cx),
            compact: s.compact_sidebar,
            show_dot: s.sidebar_status_dot,
            dot_right: s.sidebar_status_position == wormhole_core::config::StatusPosition::Right,
            show_agent: s.sidebar_agent_icon,
            show_branch: s.sidebar_branch,
            show_changes: s.sidebar_changes,
            show_notes: s.sidebar_notes,
            hints: ws.hints_showing(cx),
            code_font: crate::fonts::code_font(cx).0,
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
}

/// Where a row sits in the listing.
pub(super) struct Place {
    pub(super) position: usize,
    pub(super) of: usize,
    pub(super) number: Option<usize>,
    /// The folder of its band, when grouped.
    pub(super) band: Option<PathBuf>,
    pub(super) grouped: bool,
    /// What it shows of its split.
    pub(super) marks: SplitMarks,
}

/// The glyph a terminal's state is drawn with.
pub(super) fn glyph_for(info: &RowInfo) -> StatusGlyph {
    match (info.agent.is_some(), info.state) {
        (_, RowState::Exited) => StatusGlyph::Shell,
        (true, RowState::Working) => StatusGlyph::Working,
        (true, RowState::NeedsYou) => StatusGlyph::NeedsYou,
        (true, RowState::Finished) => StatusGlyph::Done,
        (true, RowState::Idle) => StatusGlyph::Idle,
        (false, _) if info.unread => StatusGlyph::Terminal { unread: true },
        (false, _) if info.command.is_some() => StatusGlyph::Terminal { unread: false },
        (false, _) => StatusGlyph::Shell,
    }
}

/// The part of a row's text that moves with time ("Working · 3m", a command's "1:12"), for
/// knowing when a tick has to redraw.
pub(super) fn time_text(info: &RowInfo, compact: bool) -> Option<String> {
    if compact {
        return None;
    }
    match (info.agent.is_some(), info.state) {
        (true, RowState::Working) if info.message.is_none() => took(info.since),
        (true, RowState::Finished) => Some(ago(info.since)),
        (false, _) => info.running.map(clock),
        _ => None,
    }
}

impl Workspace {
    /// The ⌘1–9 keycaps are showing (⌘ held alone long enough, and the setting on).
    pub(super) fn hints_showing(&self, cx: &App) -> bool {
        settings(cx).sidebar_cmd_hints && self.sidebar.hold.showing(Instant::now())
    }

    pub(super) fn render_row(
        &self,
        info: &RowInfo,
        place: &Place,
        listing: &Listing,
        frame: &Frame,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = frame.ui;
        let (ix, id) = (info.ix, info.id);
        let selected = ix == self.active;
        let group = SharedString::from("terminal-row");

        // The lead column and line 1's right side: the status glyph in one of them, the
        // agent's logo in the lead column when asked for (only while the agent is idle if the
        // glyph is there too, since state beats identity).
        let glyph = || style::status_glyph(("status", id as u64), ui, glyph_for(info), cx);
        let logo = info.agent.filter(|_| frame.show_agent).map(|agent| {
            let agent = agents::agent(agent);
            let color = if selected { ui.selected_text } else { ui.muted };
            style::agent_logo(agent.icon(), agent.monogram, style::ITEM_LEAD_W, color)
        });
        let agent_idle = info.agent.is_some() && info.state == RowState::Idle;
        let (lead, at_right): (AnyElement, Option<AnyElement>) = match (frame.show_dot, frame.dot_right, logo)
        {
            (true, false, Some(logo)) if agent_idle => (logo, None),
            (true, false, _) => (glyph(), None),
            (true, true, logo) => (logo.unwrap_or_else(|| div().into_any_element()), Some(glyph())),
            (false, _, logo) => (logo.unwrap_or_else(|| div().into_any_element()), None),
        };
        let bell = info.bell.then(|| style::status_glyph(("bell", id as u64), ui, StatusGlyph::Bell, cx));
        // Row details: the notes and tasks still open, the row's own changed files.
        let quiet = if selected { ui.selected_muted } else { ui.muted };
        let notes = (frame.show_notes && info.open_items > 0).then(|| {
            div()
                .flex()
                .items_center()
                .gap_1()
                .text_caption()
                .tabular_nums()
                .text_color(quiet)
                .child(Icon::new(IconName::StickyNote).xsmall())
                .child(info.open_items.to_string())
        });
        let changes = info
            .changes
            .filter(|_| frame.show_changes)
            .map(|n| div().text_caption().tabular_nums().text_color(quiet).child(format!("±{n}")));
        // What a split says of this terminal, at the right of its first line.
        let chain = place.marks.chain.then(|| style::chain_mark(("chain", id as u64), ui));
        let on_screen = place.marks.on_screen.then(|| style::on_screen_mark(ui));
        let trail = (notes.is_some()
            || changes.is_some()
            || bell.is_some()
            || at_right.is_some()
            || chain.is_some()
            || on_screen.is_some())
        .then(|| {
            div()
                .flex()
                .items_center()
                .gap_2()
                // The close × takes this place under the pointer.
                .group_hover(group.clone(), |s| s.invisible())
                .children(chain)
                .children(on_screen)
                .children(notes)
                .children(changes)
                .children(bell)
                .children(at_right)
                .into_any_element()
        });

        // The second line: every comfortable row has one. What the terminal
        // has to report comes first; a quiet one says where it is, and where even that is
        // unknown it says what it is, so no row is a single line.
        let branch = info.branch.as_deref().filter(|_| frame.show_branch);
        let whereabouts =
            || place_detail(info.cwd.as_deref(), place.band.as_deref(), branch, home_dir(), PLACE_CHARS);
        let (detail, tone, mono, detail_trail) = if frame.compact {
            (None, RowTone::Plain, false, None)
        } else if info.agent.is_some() {
            match agent_detail(info.state, info.message.as_deref(), info.since) {
                Some(text) => (Some(text), tone_of(info.state), false, None),
                None => match whereabouts() {
                    Some(place) => (Some(place), RowTone::Plain, true, None),
                    None => (Some("Idle".to_string()), RowTone::Plain, false, None),
                },
            }
        } else {
            let trail = if info.unread { Some("new output".to_string()) } else { info.running.map(clock) };
            match &info.command {
                Some(command) => (Some(command.clone()), RowTone::Plain, true, trail),
                None => match whereabouts() {
                    Some(place) => (Some(place), RowTone::Plain, true, None),
                    None => (Some("Shell".to_string()), RowTone::Plain, false, None),
                },
            }
        };

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
                    .map(|f| wormhole_core::sidebar::display_path(f, home_dir()))
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
        let details = (frame.show_notes && info.open_items > 0) as u8 + info.changes.is_some() as u8;
        let right = match (&hint, frame.dot_right || info.bell) {
            (Some(_), _) => KEYCAP_TRAIL_W,
            (None, true) => TRAIL_W,
            (None, false) => 0.0,
        } + f32::from(details) * DETAIL_W
            + if place.marks.chain { TRAIL_W } else { 0.0 }
            + if place.marks.on_screen { ON_SCREEN_W } else { 0.0 };
        let name_room = frame.width
            - 2.0 * style::SIDEBAR_INSET
            - style::ITEM_LEAD_W
            - 8.0
            - right
            - marks as f32 * MARK_W;
        let name_weight = if selected || info.unread { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM };
        let cut_name = frame.measure(&info.name, BODY_REM, name_weight, window) > name_room;
        let detail_room = frame.width
            - 2.0 * style::SIDEBAR_INSET
            - style::ITEM_LEAD_W
            - 8.0
            - detail_trail.as_ref().map_or(0.0, |_| CLOCK_W);
        let cut_detail = !mono
            && detail
                .as_deref()
                .is_some_and(|d| frame.measure(d, CAPTION_REM, FontWeight::MEDIUM, window) > detail_room)
            || mono
                && detail.as_deref().is_some_and(|d| d.chars().count() as f32 * MONO_CHAR_W > detail_room);
        let tooltip = (cut_name || cut_detail).then(|| {
            let where_ = place
                .band
                .as_deref()
                .or(info.cwd.as_deref())
                .map(|folder| wormhole_core::sidebar::display_path(folder, home_dir()));
            let second = [where_, detail.clone()].into_iter().flatten().collect::<Vec<_>>().join(" · ");
            (info.name.clone(), SharedString::from(second))
        });

        let label = spoken::row_label_in_split(
            &info.name,
            match info.agent {
                Some(_) => RowSpeech::Agent {
                    state: info.state,
                    detail: info.message.as_deref().filter(|_| info.state != RowState::Idle),
                },
                None => RowSpeech::Plain {
                    command: info.command.as_deref(),
                    running: info.running,
                    bell: info.bell,
                    unread: info.unread,
                },
            },
            spoken::Place { selected, position: place.position, of: place.of, number: place.number },
            SplitSpeech::of(place.marks),
        );

        let row = style::session_row(
            ("session", id as u64),
            ui,
            SessionRow {
                glyph: lead,
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
            },
        );
        let flash = self.sidebar.flash.filter(|(flashed, _)| *flashed == id).map(|(_, generation)| {
            div().absolute().top_0().left_0().size_full().bg(ui.highlight).with_animation(
                ("move-flash", generation),
                Animation::new(MOVE_FLASH),
                |d, t| d.opacity(1.0 - t),
            )
        });
        let dragged = cx.has_active_drag()
            && self
                .terminal_drag
                .as_ref()
                .is_some_and(|drag| drag.id == id && drag.origin == DragOrigin::Row);
        // The keys' place: VoiceOver follows it, and the ring shows while keys move it.
        let (keyed, ring) = self.sidebar_keyed(&SidebarStop::Row(id), window);
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
            .on_click(cx.listener(move |this, _, window, cx| this.activate(ix, window, cx)))
            .when_some(tooltip, |d, (name, second)| {
                d.tooltip(move |window, cx| two_line_tooltip(name.clone(), second.clone(), ui, window, cx))
            })
            .children(flash)
            .children(place.marks.bracket.map(|part| style::split_bracket(ui, part)))
            .when(self.sidebar.menu_row == Some(id) || ring, |d| {
                d.child(style::focus_ring(ui, style::RingPlacement::Inside, px(0.0)))
            })
            .children(self.drop_line(info, listing, cx.has_active_drag(), ui))
            .when(!frame.hints, |d| d.child(self.close_button(info, selected, &group, ui, cx)))
            .map(|d| self.with_drag(d, info, cx));
        // Last: the menu wraps the row.
        self.with_row_menu(row, info, cx).into_any_element()
    }

    /// The × that closes a terminal: on the row under the pointer, the selected one too.
    fn close_button(
        &self,
        info: &RowInfo,
        selected: bool,
        group: &SharedString,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ix = info.ix;
        let fill = if selected { style::SelectedFill::Tint } else { style::SelectedFill::Plain };
        // No tooltip while a close question is up: the pointer still rests on the ×, and GPUI
        // would show (or keep) its tooltip beside the dialog. Dropping it clears what is pending.
        let asking = self.closing_session_prompt || self.closing_prompt;
        div()
            .absolute()
            .top(px(CLOSE_X_TOP))
            .right(px(CLOSE_X_RIGHT))
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
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.request_close_session(ix, window, cx);
                        }))
                },
            ))
            .into_any_element()
    }
}

/// The second line's color for an agent's state.
fn tone_of(state: RowState) -> RowTone {
    match state {
        RowState::NeedsYou => RowTone::Attention,
        RowState::Finished => RowTone::Done,
        RowState::Working | RowState::Idle | RowState::Exited => RowTone::Plain,
    }
}

/// A code-font character at caption size, roughly, and the room a clock takes.
const MONO_CHAR_W: f32 = 7.2;
const CLOCK_W: f32 = 48.0;
/// How many characters of a path fit on a row's second line before it is cut from the front.
const PLACE_CHARS: usize = 38;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use wormhole_core::agents::AgentId;
    use wormhole_core::sidebar::RowState;

    use super::{RowInfo, glyph_for, time_text};
    use crate::style::StatusGlyph;

    fn info(agent: bool, state: RowState) -> RowInfo {
        RowInfo {
            ix: 0,
            id: 1,
            name: "t".into(),
            agent: agent.then_some(AgentId::Claude),
            state,
            message: None,
            since: Duration::from_secs(180),
            cwd: Some(PathBuf::from("/w")),
            folder: None,
            worktree: None,
            branch: None,
            command: None,
            running: None,
            bell: false,
            unread: false,
            pinned: false,
            open_items: 0,
            changes: None,
        }
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
    fn only_moving_times_count_for_redraws() {
        assert_eq!(time_text(&info(true, RowState::Working), false).as_deref(), Some("3m"));
        let told = RowInfo { message: Some("Editing".into()), ..info(true, RowState::Working) };
        assert_eq!(time_text(&told, false), None, "the agent's own words don't move");
        assert_eq!(time_text(&info(true, RowState::Finished), false).as_deref(), Some("3m ago"));
        let running = RowInfo { running: Some(Duration::from_secs(72)), ..info(false, RowState::Working) };
        assert_eq!(time_text(&running, false).as_deref(), Some("1:12"));
        assert_eq!(time_text(&running, true), None, "compact rows have no second line");
    }
}

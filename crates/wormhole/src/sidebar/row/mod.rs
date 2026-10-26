//! One terminal's row: the lead glyph or agent logo, the name
//! at x 40, the second line (what the agent does, the command in front, where it is), what
//! sits at x 334 (⌘N keycap, status glyph on the right, bell, close ×), and the states laid
//! over it (its context menu's focus ring, a drag, the flash after a move). A terminal in a
//! split also shows it: the chain mark at the right of its first line when its mates are elsewhere
//! (rows of one split in no group share a card; VoiceOver is told they are on screen).

mod face;
mod frame;
mod label;
mod second_line;
mod tile;

use std::time::Duration;

use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::sidebar::hints::keyshortcut;

use self::face::RowFace;
pub(super) use self::face::{PLACE_CHARS, Place, row_face};
pub(super) use self::frame::Frame;
#[cfg(test)]
use self::label::row_label;
pub(super) use self::second_line::time_text;
#[cfg(test)]
use self::second_line::{SecondLine, second_line};
pub(super) use self::tile::{glyph_for, tile_for, tile_parts};

use super::facts::RowInfo;
use super::{Sidebar, SidebarEvent, two_line_tooltip};
use crate::settings::UiColors;
use crate::sidebar::SidebarStop;
use crate::sidebar::terminal_drag::DragOrigin;
use crate::style;

/// How long the row that moved to another band stays highlighted, fading out.
const MOVE_FLASH: Duration = Duration::from_millis(600);
/// The close ×: drawn 22 square, this far under the row's top.
const CLOSE_X_TOP: f32 = 3.0;

/// How far the × stands in from the row's right side: its glyph ends where line 1 ends (the row's
/// own padding, and a lone card's), on the right edge of what sits there at rest.
fn close_right(metrics: &style::Metrics, solo: bool) -> f32 {
    let line_end = metrics.row_px + if solo { metrics.card_pad } else { 0.0 };
    line_end - (style::CONTROL_H_SM - style::GLYPH) / 2.0
}

impl Sidebar {
    pub(in crate::sidebar) fn render_row(
        &self,
        info: &RowInfo,
        place: &Place,
        frame: &Frame,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = frame.ui;
        let id = info.id;
        let selected = place.selected;
        let RowFace { row, tooltip, label, group } =
            row_face(info, place, frame, self.rename_field(id, cx), window, cx);
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
    pub(in crate::sidebar) fn close_button(
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
            selected: false,
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

//! Snapshot tests: every step at several sizes, as the plain text of its rows, and the entrance as it
//! types out.

use std::path::Path;
use std::time::Duration;

use super::ansi::Style;
use super::art::{self, LOGO, MARK_CELL};
use super::facts;
use super::fixtures;
use super::flow::Step;
use super::keys::Key;
use super::model::Welcome;
use super::render::{LINE_PAUSE, ROW_EVERY, Size, TYPE_NAME, TYPE_TEXT, Target, render};
use crate::config::NotifySound;
use crate::intro::Handover;
use crate::ipc::control::{COMMAND_NAME, PermissionFact};
use crate::keymap::{self, ActionId};

const END: Duration = Duration::MAX;
const NOW: Duration = Duration::ZERO;

fn rows(w: &Welcome, cols: usize, rows: usize, clock: Duration) -> Vec<String> {
    render(w, Size { cols, rows }, clock).grid.text()
}

/// The page above the footer, without the blank rows at its end.
fn page(w: &Welcome, cols: usize, height: usize, clock: Duration) -> Vec<String> {
    let mut all = rows(w, cols, height, clock);
    all.pop();
    while all.last().is_some_and(String::is_empty) {
        all.pop();
    }
    all
}

fn footer(w: &Welcome, cols: usize, height: usize) -> String {
    rows(w, cols, height, END).pop().unwrap_or_default()
}

/// The first run's welcome, moved to its `step` the way a person would.
fn at(step: Step, mut w: Welcome) -> Welcome {
    for _ in 0..5 {
        if w.step() == step {
            break;
        }
        w.key(Key::Enter, NOW);
    }
    assert_eq!(w.step(), step);
    w
}

fn lines(text: &[&str]) -> Vec<String> {
    text.iter().map(ToString::to_string).collect()
}

/// The fixture's state with macOS' permission as `permission` says.
fn with_permission(permission: PermissionFact) -> Welcome {
    let mut state = fixtures::state(Handover::Cursor);
    if let Some(notify) = state.notify.as_mut() {
        notify.permission = permission;
    }
    Welcome::new(state, true)
}

// -------------------------------------------------------------------------------------- the pages

#[test]
fn the_welcome_page_points_at_the_sidebar() {
    let expected = lines(&[
        "",
        "  >_ Wormhole",
        "     A terminal for working with AI coding agents.",
        "",
        "  ←  your terminals and what their agents are doing",
        "  ←  which is working, which needs you, which is done",
        "  ←  ⌘T opens another; click one to switch",
        "",
        "  ❯ Get started",
    ]);
    for handover in [Handover::Cursor, Handover::Portal] {
        let w = fixtures::welcome(handover);
        assert_eq!(page(&w, 80, 24, END), expected, "{handover:?}");
        assert_eq!(footer(&w, 80, 24), "  ⏎ get started  esc skip");
    }
}

#[test]
fn the_logo_page_has_a_block_art_logo_and_the_name_beside_it() {
    let w = fixtures::welcome(Handover::Logo);
    assert_eq!(
        page(&w, 80, 24, END),
        lines(&[
            "",
            "  ▄          ▄",
            "    ▀ ▀         Wormhole",
            "     ▀ ▀        A terminal for working with AI coding agents.",
            "    ▀▀   ▀▀▀",
            "  ▀          ▀",
            "",
            "  ←  your terminals and what their agents are doing",
            "  ←  which is working, which needs you, which is done",
            "  ←  ⌘T opens another; click one to switch",
            "",
            "  ❯ Get started",
        ])
    );
}

#[test]
fn a_replay_is_the_welcome_page_alone_and_ends_on_return() {
    let mut state = fixtures::state(Handover::Cursor);
    state.first_run = false;
    let w = Welcome::new(state, true);
    assert_eq!(page(&w, 80, 24, END).last().map(String::as_str), Some("  ❯ Back to Wormhole"));
    assert_eq!(footer(&w, 80, 24), "  ⏎ back to Wormhole  esc skip");
}

#[test]
fn the_look_step_offers_themes_a_colour_a_font_and_ligatures() {
    let w = at(Step::Look, fixtures::welcome(Handover::Cursor));
    assert_eq!(
        page(&w, 80, 24, END),
        lines(&[
            "",
            "  How should it look?",
            "  Themes apply as you move. The rest of it is in Settings.",
            "",
            "  Theme",
            "  ❯ Tokyo Night",
            "    Catppuccin Mocha",
            "    Nord",
            "    Rose Pine",
            "    Gruvbox Dark",
            "    GitHub Light  light",
            "",
            "  Colour     ● Theme  ○ Green  ○ Magenta  ○ Red  ○ Yellow  ○ Cyan",
            "  Font       Menlo 13",
            "  Ligatures  on   => != >=",
        ])
    );
    assert_eq!(
        footer(&w, 80, 24),
        "  ↑↓ choose  p colour  l ligatures  + - size  ⏎ continue  ← back  esc skip"
    );
}

#[test]
fn a_short_window_shrinks_the_theme_list_around_the_cursor() {
    let mut w = at(Step::Look, fixtures::welcome(Handover::Cursor));
    for _ in 0..4 {
        w.key(Key::Down, NOW);
    }
    let text = page(&w, 40, 14, END);
    // Two theme rows fit; the cursor is on the fifth, so the window has moved down to it.
    assert!(text.contains(&"  ❯ Gruvbox Dark".to_string()), "{text:#?}");
    assert!(!text.contains(&"    Tokyo Night".to_string()));
    assert_eq!(text.last().map(String::as_str), Some("  Ligatures  on   => != >="));
}

#[test]
fn the_notifications_step_offers_the_settings_switches_with_the_current_ones_chosen() {
    let w = at(Step::Notify, fixtures::welcome(Handover::Cursor));
    assert_eq!(
        page(&w, 80, 24, END),
        lines(&[
            "",
            "  Tell me when an agent needs me",
            "  A banner and a sound can tell you when a terminal needs you or an agent",
            "  finishes, so you can look away. The same switches are in Settings (⌘,).",
            "",
            "    Needs you  ● On  ○ Off    an agent asks a question or for permission",
            "    Finished   ● On  ○ Off    an agent is done",
            "    Sound      ● Needs you only  ○ All  ○ None",
            "    Allow notifications…  macOS asks once, and only when you press this",
            "",
            "  ❯ Continue",
        ])
    );
    // The cursor starts on the button, so Return goes on and presses nothing.
    assert_eq!(footer(&w, 80, 24), "  ↑↓ choose  ⏎ continue  ← back  esc skip");
}

#[test]
fn the_notifications_step_shows_what_was_chosen_and_names_what_return_does() {
    let mut w = at(Step::Notify, fixtures::welcome(Handover::Cursor));
    // Up from Continue: Allow, Sound, Finished, Needs you.
    w.key(Key::Up, NOW);
    assert_eq!(footer(&w, 80, 24), "  ↑↓ choose  ⏎ allow  ← back  esc skip");
    w.key(Key::Up, NOW);
    assert_eq!(footer(&w, 80, 24), "  ↑↓ choose  ⏎ change  ← back  esc skip");
    w.key(Key::Enter, NOW);
    let text = page(&w, 80, 24, END);
    assert!(text.contains(&"  ❯ Sound      ○ Needs you only  ● All  ○ None".to_string()), "{text:#?}");
    assert_eq!(w.notify().map(|notify| notify.sound), Some(NotifySound::All));
    w.key(Key::Up, NOW);
    w.key(Key::Enter, NOW);
    let text = page(&w, 80, 24, END);
    assert!(text.contains(&"  ❯ Finished   ○ On  ● Off    an agent is done".to_string()), "{text:#?}");
}

#[test]
fn what_macos_says_replaces_the_allow_button() {
    let allowed = at(Step::Notify, with_permission(PermissionFact::Allowed));
    let text = page(&allowed, 80, 24, END);
    assert!(text.contains(&"  ✓ macOS lets Wormhole show banners.".to_string()), "{text:#?}");
    assert!(text.iter().all(|row| !row.contains("Allow notifications")));
    let denied = at(Step::Notify, with_permission(PermissionFact::Denied));
    let text = page(&denied, 80, 24, END);
    assert!(text.iter().any(|row| row.starts_with("  ! Off in System Settings")), "{text:#?}");
    assert!(text.iter().all(|row| !row.contains("Allow notifications")), "only the person can turn it on");
    let outside = at(Step::Notify, with_permission(PermissionFact::Outside));
    let text = page(&outside, 80, 24, END);
    assert!(text.iter().any(|row| row.contains("outside Wormhole.app")), "{text:#?}");
}

#[test]
fn the_sidebar_step_points_at_the_example_rows_and_lists_each_agent_with_its_hooks() {
    let w = at(Step::Sidebar, fixtures::welcome(Handover::Cursor));
    assert_eq!(
        page(&w, 80, 24, END),
        lines(&[
            "",
            "  Watch the sidebar",
            "  The Example folder in the sidebar is only there while this page is up: three",
            "  terminals showing what a row tells you.",
            "",
            "  ←  working    the agent is busy; nothing for you to do",
            "  ←  needs you  it asked a question or for permission",
            "  ←  done       it finished; time to look at what it did",
            "",
            "  Coding agents",
            "  Hooks tell Wormhole what an agent is doing. Nothing is installed until you",
            "  press Install on its row.",
            "",
            "    Claude Code  ~/.claude/settings.json                           [ Install ]",
            "    Codex CLI    ~/.codex/hooks.json                               Installed ✓",
            "    Aider                                               status from its output",
            "    11 more recognised, not found on PATH",
            "",
            "  ❯ Continue",
        ])
    );
    assert_eq!(footer(&w, 80, 24), "  ↑↓ choose  ⏎ continue  ← back  esc skip");
    // On a narrow window the paths go and the rows still fit.
    let narrow = page(&w, 50, 30, END);
    assert!(narrow.contains(&"    Claude Code                      [ Install ]".to_string()), "{narrow:#?}");
}

#[test]
fn the_cursor_on_an_installable_agent_names_what_return_does() {
    let mut w = at(Step::Sidebar, fixtures::welcome(Handover::Cursor));
    for _ in 0..3 {
        w.key(Key::Up, NOW);
    }
    assert_eq!(footer(&w, 80, 24), "  ↑↓ choose  ⏎ install  ← back  esc skip");
    assert!(page(&w, 80, 24, END).contains(
        &"  ❯ Claude Code  ~/.claude/settings.json                           [ Install ]".to_string()
    ));
}

#[test]
fn while_the_agents_are_looked_for_the_step_says_so() {
    let mut state = fixtures::state(Handover::Cursor);
    state.agents = None;
    let w = at(Step::Sidebar, Welcome::new(state, true));
    assert!(page(&w, 80, 24, END).contains(&"  Looking for agents…".to_string()));
}

#[test]
fn without_an_agent_the_sidebar_step_has_the_primer_in_place_of_the_list() {
    let mut state = fixtures::state(Handover::Cursor);
    state.agents = Some(Default::default());
    let w = at(Step::Sidebar, Welcome::new(state, true));
    assert_eq!(
        page(&w, 80, 24, END),
        lines(&[
            "",
            "  Watch the sidebar",
            "  The Example folder in the sidebar is only there while this page is up: three",
            "  terminals showing what a row tells you.",
            "",
            "  ←  working    the agent is busy; nothing for you to do",
            "  ←  needs you  it asked a question or for permission",
            "  ←  done       it finished; time to look at what it did",
            "",
            "  No coding agent found on PATH. Install Claude Code, Codex or another and",
            "  Wormhole recognises it in any terminal, then offers its hooks once.",
            "",
            "  ❯ Continue",
        ])
    );
    assert_eq!(footer(&w, 80, 24), "  ⏎ continue  ← back  esc skip");
}

#[test]
fn an_install_that_failed_is_said_above_the_footer() {
    let mut state = fixtures::state(Handover::Cursor);
    state.problem = Some("Couldn't write the hooks: permission denied".to_string());
    let w = at(Step::Sidebar, Welcome::new(state, true));
    let all = rows(&w, 80, 24, END);
    assert_eq!(all[21], "  Couldn't write the hooks: permission denied");
    assert_eq!(w.facts().problem.as_deref(), Some("Couldn't write the hooks: permission denied"));
}

#[test]
fn the_last_step_lists_what_else_is_here_with_the_keymaps_keys() {
    let w = at(Step::Extras, fixtures::welcome(Handover::Cursor));
    let (changes, notes, palette) = (
        keymap::glyphs(ActionId::ShowChanges),
        keymap::glyphs(ActionId::ShowNotes),
        keymap::glyphs(ActionId::CommandPalette),
    );
    // The keys are the keymap's, whatever they are bound to.
    assert!(!changes.is_empty() && !notes.is_empty() && !palette.is_empty());
    assert_eq!(
        page(&w, 80, 24, END),
        vec![
            String::new(),
            "  What else is here".to_string(),
            format!("  Everything has a key, and every action is in the command palette ({palette})."),
            String::new(),
            format!("  {changes}  Changes    what an agent changed. Click a line of a diff to comment on"),
            "                  it, then paste the comments into the terminal for its agent.".to_string(),
            format!("  {notes}  Notes      notes and next steps for each terminal, kept with it."),
            format!("  {palette}  Worktrees  New Terminal in New Worktree… gives a branch a folder and a"),
            "                  terminal of its own.".to_string(),
            format!(
                "  {COMMAND_NAME}   Scripts    {COMMAND_NAME} help, in any of these terminals, lists what scripts and"
            ),
            format!(
                "                  agents can ask of Wormhole. Settings → Coding agents puts {COMMAND_NAME}"
            ),
            "                  in other terminals too.".to_string(),
            String::new(),
            "  ❯ Start".to_string(),
        ]
    );
    assert_eq!(footer(&w, 80, 24), "  ⏎ start  ← back  esc skip");
}

// -------------------------------------------------------------------------------- the read-only tour

#[test]
fn a_tour_says_once_that_it_is_read_only_and_offers_no_install() {
    let w = Welcome::new(facts::tour_state(Path::new("/nonexistent")), false);
    let all = rows(&w, 80, 24, END);
    assert_eq!(all[22], "  Read-only tour: not connected to Wormhole, so nothing here changes anything.");
    assert_eq!(all.iter().filter(|row| row.contains("Read-only")).count(), 1, "one line");
    // Its sidebar step shows no Install button, whatever the machine has.
    let mut state = fixtures::state(Handover::Cursor);
    state.look.themes.clear();
    state.notify = None;
    let tour = at(Step::Sidebar, Welcome::new(state, false));
    let text = page(&tour, 80, 24, END);
    assert!(text.iter().all(|row| !row.contains("[ Install ]")), "{text:#?}");
    assert!(text.iter().any(|row| row.contains("not installed")));
    let look = at(Step::Look, Welcome::new(facts::tour_state(Path::new("/nonexistent")), false));
    assert!(page(&look, 80, 24, END).iter().any(|row| row.contains("read-only")));
    let notify = at(Step::Notify, Welcome::new(facts::tour_state(Path::new("/nonexistent")), false));
    let text = page(&notify, 80, 24, END);
    assert!(text.iter().any(|row| row.contains("read-only")), "{text:#?}");
    assert!(text.iter().all(|row| !row.contains("Needs you")), "no switches to press");
}

#[test]
fn a_live_welcome_has_no_tour_line() {
    let w = fixtures::welcome(Handover::Cursor);
    assert!(rows(&w, 80, 24, END).iter().all(|row| !row.contains("Read-only")));
}

// ---------------------------------------------------------------------------------- the entrance

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// The Cursor hand-over: the `>` is there when the ship has docked, the `_` a beat after, and then the
/// name types a few milliseconds a character.
#[test]
fn the_cursor_hand_over_shows_the_mark_then_types_the_name() {
    let w = fixtures::welcome(Handover::Cursor);
    let row1 = |clock| rows(&w, 80, 24, clock)[1].clone();
    assert_eq!(row1(NOW), "  >");
    assert_eq!(row1(ROW_EVERY), "  >_");
    let name_from = ROW_EVERY * 2;
    assert_eq!(row1(name_from), "  >_ W");
    assert_eq!(row1(name_from + TYPE_NAME * 3), "  >_ Worm");
    assert_eq!(row1(name_from + TYPE_NAME * 8), "  >_ Wormhole");
    // The tagline waits for the name; the footer waits for everything.
    assert_eq!(rows(&w, 80, 24, name_from + TYPE_NAME * 3)[2], "");
    assert_eq!(rows(&w, 80, 24, ms(300))[23], "");
    assert_eq!(rows(&w, 80, 24, END)[23], "  ⏎ get started  esc skip");
}

#[test]
fn the_portal_hand_over_types_the_mark_with_the_name() {
    let w = fixtures::welcome(Handover::Portal);
    let row1 = |clock| rows(&w, 80, 24, clock)[1].clone();
    assert_eq!(row1(NOW), "  >");
    assert_eq!(row1(TYPE_NAME), "  >_");
    assert_eq!(row1(TYPE_NAME * 4), "  >_ Wo");
    assert_eq!(row1(TYPE_NAME * 20), "  >_ Wormhole");
}

#[test]
fn the_logo_hand_over_has_the_logo_there_at_once_and_types_the_name_beside_it() {
    let w = fixtures::welcome(Handover::Logo);
    let at_zero = rows(&w, 80, 24, NOW);
    assert_eq!(at_zero[1], "  ▄          ▄", "the logo is whole when the ship has docked");
    assert_eq!(at_zero[2], "    ▀ ▀", "…with nothing beside it yet");
    let typing = rows(&w, 80, 24, ROW_EVERY + TYPE_NAME * 3);
    assert_eq!(typing[2], "    ▀ ▀         Worm");
}

#[test]
fn list_rows_appear_one_by_one() {
    let w = at(Step::Look, fixtures::welcome(Handover::Cursor));
    // Past the heading, its line and the "Theme" label, each row a ROW_EVERY after the one before.
    let heading = TYPE_NAME * 19 + LINE_PAUSE;
    let note = TYPE_TEXT * 56 + LINE_PAUSE;
    let first = heading + note + ROW_EVERY;
    let listed =
        |clock: Duration| rows(&w, 80, 24, clock)[5..11].iter().filter(|row| !row.is_empty()).count();
    assert_eq!(listed(first - Duration::from_millis(1)), 0);
    assert_eq!(listed(first), 1);
    assert_eq!(listed(first + ROW_EVERY), 2);
    assert_eq!(listed(first + ROW_EVERY * 5), 6);
    assert_eq!(listed(END), 6);
}

/// A key cuts the entrance short, and a step already seen comes in whole.
#[test]
fn a_key_settles_the_page_and_a_step_seen_comes_back_whole() {
    let mut w = fixtures::welcome(Handover::Cursor);
    assert_eq!(rows(&w, 80, 24, NOW)[4], "", "the first page is still coming in");
    w.key(Key::Char('?'), NOW);
    assert_eq!(rows(&w, 80, 24, NOW)[4], "  ←  your terminals and what their agents are doing");
    // The next step is new and types in; going back to this one shows it whole.
    w.key(Key::Enter, NOW);
    assert_eq!(rows(&w, 80, 24, NOW)[4], "", "the theme list has not come in");
    w.key(Key::Left, ms(10));
    assert_eq!(w.step(), Step::Welcome);
    assert_eq!(rows(&w, 80, 24, ms(10))[4], "  ←  your terminals and what their agents are doing");
}

#[test]
fn under_reduce_motion_nothing_types() {
    let mut state = fixtures::state(Handover::Cursor);
    state.reduce_motion = true;
    let w = Welcome::new(state, true);
    assert_eq!(rows(&w, 80, 24, NOW), rows(&w, 80, 24, END));
    assert_eq!(rows(&w, 80, 24, NOW)[1], "  >_ Wormhole");
}

// ---------------------------------------------------------------------------- the fixed places

/// The ship is aimed at fixed cells: the page puts the mark's `>` and the logo there at any size
/// that has room for them.
#[test]
fn the_mark_and_the_logo_are_where_the_animation_aims() {
    let (row, col) = MARK_CELL;
    for handover in [Handover::Cursor, Handover::Portal] {
        let w = fixtures::welcome(handover);
        for (cols, height) in [(80, 24), (100, 40), (40, 12), (120, 30), (24, 6)] {
            let frame = render(&w, Size { cols, rows: height }, END);
            let line = frame.grid.text_row(row);
            assert_eq!(line.chars().nth(col), Some('>'), "{handover:?} at {cols}x{height}: {line:?}");
            assert_eq!(line.chars().nth(col + 1), Some('_'));
        }
    }
    let w = fixtures::welcome(Handover::Logo);
    for (cols, height) in [(80, 24), (100, 40), (40, 12), (120, 30)] {
        let frame = render(&w, Size { cols, rows: height }, END);
        for r in 0..LOGO.rows {
            for c in 0..LOGO.cols {
                let (ch, _) = art::logo_cell(r, c);
                let drawn = frame.grid.text_row(LOGO.row + r).chars().nth(LOGO.col + c).unwrap_or(' ');
                if ch != ' ' {
                    assert_eq!(drawn, ch, "logo cell ({r},{c}) at {cols}x{height}");
                }
            }
        }
    }
}

#[test]
fn the_logo_is_drawn_with_the_palette_and_the_chevron_is_bright() {
    let w = fixtures::welcome(Handover::Logo);
    let frame = render(&w, Size { cols: 80, rows: 24 }, END);
    // The chevron's first pixel: bright white over the tile's grey.
    let style = frame.grid.style_at(LOGO.row + 1, LOGO.col + 2);
    assert_eq!((style.fg, style.bg), (Some(15), Some(8)));
}

// ---------------------------------------------------------------------------------- the sweeps

/// Every step at many sizes, down to nothing: no panic, the footer is the last row when there is room,
/// and no cell is outside the palette.
#[test]
fn every_step_draws_at_every_size_with_palette_colours_only() {
    for handover in Handover::ALL {
        for permission in [PermissionFact::NotAsked, PermissionFact::Allowed, PermissionFact::Denied] {
            let mut state = fixtures::state(handover);
            if let Some(notify) = state.notify.as_mut() {
                notify.permission = permission;
            }
            let mut w = Welcome::new(state, true);
            let mut seen = Vec::new();
            for _ in 0..6 {
                seen.push(w.step());
                for cols in [0, 1, 7, 20, 40, 60, 80, 120, 200] {
                    for height in [0, 1, 2, 3, 6, 10, 14, 24, 50] {
                        let frame = render(&w, Size { cols, rows: height }, END);
                        assert_eq!(frame.grid.rows(), height);
                        for r in 0..height {
                            for c in 0..cols {
                                let Style { fg, bg, .. } = frame.grid.style_at(r, c);
                                assert!(fg.is_none_or(|n| n < 16) && bg.is_none_or(|n| n < 16));
                            }
                        }
                        if cols >= 40 && height >= 6 {
                            assert!(
                                !frame.grid.text_row(height - 1).is_empty(),
                                "{:?} {cols}x{height}",
                                w.step()
                            );
                        }
                        // Entrances at every moment too.
                        for clock in [NOW, ms(37), ms(900)] {
                            let _ = render(&w, Size { cols, rows: height }, clock);
                        }
                    }
                }
                w.key(Key::Enter, NOW);
            }
            for step in [Step::Welcome, Step::Look, Step::Notify, Step::Sidebar, Step::Extras] {
                assert!(seen.contains(&step), "{handover:?} {seen:?}");
            }
        }
    }
}

#[test]
fn the_footer_and_the_button_stay_on_a_window_too_short_for_the_page() {
    for step in [Step::Look, Step::Notify, Step::Sidebar, Step::Extras] {
        let w = at(step, fixtures::welcome(Handover::Cursor));
        let all = rows(&w, 60, 12, END);
        assert!(all[11].contains('⏎'), "{step:?} footer: {:?}", all[11]);
        if step != Step::Look {
            let button = if step == Step::Extras { "Start" } else { "Continue" };
            assert!(all.iter().any(|row| row.contains(button)), "{step:?} has lost its button: {all:#?}");
        }
    }
}

#[test]
fn the_pages_do_not_depend_on_which_handover_after_the_first() {
    let mut cursor = fixtures::welcome(Handover::Cursor);
    let mut logo = fixtures::welcome(Handover::Logo);
    for _ in 0..4 {
        cursor.key(Key::Enter, NOW);
        logo.key(Key::Enter, NOW);
        assert_eq!(rows(&cursor, 80, 24, END), rows(&logo, 80, 24, END));
    }
}

// ------------------------------------------------------------------------------------------ clicks

#[test]
fn rows_and_footer_hints_can_be_clicked() {
    let w = at(Step::Notify, fixtures::welcome(Handover::Cursor));
    let frame = render(&w, Size { cols: 80, rows: 24 }, END);
    let hit =
        |row: usize, col: usize| frame.hits.iter().find(|hit| hit.contains(row, col)).map(|hit| hit.target);
    // The three switches and Allow, from row 5, anywhere along the row; then the button, past a blank.
    for (line, row) in (5..9).enumerate() {
        assert_eq!(hit(row, 6), Some(Target::Row(line)), "row {row}");
    }
    assert_eq!(hit(5, 20), Some(Target::Row(0)), "on the choices too");
    assert_eq!(hit(9, 6), None);
    assert_eq!(hit(10, 6), Some(Target::Row(4)));
    // The footer: back, skip; "choose" has no key of its own.
    let footer_row = 23;
    let back = frame
        .grid
        .text_row(footer_row)
        .find('←')
        .map(|byte| frame.grid.text_row(footer_row)[..byte].chars().count());
    assert_eq!(hit(footer_row, back.unwrap_or(0)), Some(Target::Key(Key::Left)));
    assert_eq!(hit(footer_row, 3), None);
}

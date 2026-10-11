//! Snapshot tests: every step at several sizes, as the plain text of its rows, and the entrance as it
//! types out.

use std::path::Path;
use std::time::Duration;

use super::ansi::Style;
use super::art::{self, LOGO, MARK_CELL};
use super::facts;
use super::fixtures;
use super::keys::Key;
use super::model::Welcome;
use super::render::{LINE_PAUSE, ROW_EVERY, Size, TYPE_NAME, Target, render};
use crate::intro::Handover;
use crate::onboarding::Step;

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
    for _ in 0..4 {
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
    let w = Welcome::new(state, true, true);
    assert_eq!(page(&w, 80, 24, END).last().map(String::as_str), Some("  ❯ Back to Wormhole"));
    assert_eq!(footer(&w, 80, 24), "  ⏎ back to Wormhole  esc skip");
}

#[test]
fn the_folder_step_lists_the_places_with_where_they_are() {
    let w = at(Step::Folder, fixtures::welcome(Handover::Cursor));
    assert_eq!(
        page(&w, 80, 24, END),
        lines(&[
            "",
            "  Where do you work?",
            "  Your shell starts in the folder you pick.",
            "",
            "  ❯ terminal   ~/Projects",
            "    Home       ~",
            "    Desktop    ~/Desktop",
            "    Documents  ~/Documents",
        ])
    );
    assert_eq!(footer(&w, 80, 24), "  ↑↓ choose  ⏎ continue  ← back  esc skip");
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
fn the_agents_step_lists_each_with_its_hooks_and_ends_on_start() {
    let w = at(Step::Agents, fixtures::welcome(Handover::Cursor));
    assert_eq!(
        page(&w, 80, 24, END),
        lines(&[
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
            "  ❯ Start",
        ])
    );
    assert_eq!(footer(&w, 80, 24), "  ↑↓ choose  ⏎ start  ← back  esc skip");
    // On a narrow window the paths go and the rows still fit.
    let narrow = page(&w, 50, 24, END);
    assert!(narrow.contains(&"    Claude Code                      [ Install ]".to_string()), "{narrow:#?}");
}

#[test]
fn the_cursor_on_an_installable_agent_names_what_return_does() {
    let mut w = at(Step::Agents, fixtures::welcome(Handover::Cursor));
    for _ in 0..3 {
        w.key(Key::Up, NOW);
    }
    assert_eq!(footer(&w, 80, 24), "  ↑↓ choose  ⏎ install  ← back  esc skip");
    assert!(page(&w, 80, 24, END).contains(
        &"  ❯ Claude Code  ~/.claude/settings.json                           [ Install ]".to_string()
    ));
}

#[test]
fn a_folder_with_a_project_offers_its_trust() {
    let mut state = fixtures::state(Handover::Cursor);
    state.trust = Some(fixtures::trust());
    let mut w = at(Step::Agents, Welcome::new(state.clone(), true, true));
    let text = page(&w, 80, 24, END);
    assert!(text.contains(&"  terminal/ has a .wormhole/ folder".to_string()), "{text:#?}");
    assert!(text.contains(&"    Trust this folder (Touch ID)…".to_string()));
    assert!(text.contains(&"  ❯ Start".to_string()), "the cursor still starts on Start");
    // The Touch ID prompt is up: the row says so.
    state.trust = state.trust.map(fixtures::trust_asking);
    w.update(state);
    assert!(page(&w, 80, 24, END).contains(&"    Waiting for approval…".to_string()));
}

#[test]
fn while_the_agents_are_looked_for_the_step_says_so() {
    let mut state = fixtures::state(Handover::Cursor);
    state.agents = None;
    let w = at(Step::Agents, Welcome::new(state, true, true));
    assert!(page(&w, 80, 24, END).contains(&"  Looking for agents…".to_string()));
}

#[test]
fn without_an_agent_the_third_step_is_a_primer_with_the_keymaps_keys() {
    let mut state = fixtures::state(Handover::Cursor);
    state.agents = Some(Default::default());
    let w = at(Step::Primer, Welcome::new(state, true, true));
    assert_eq!(
        page(&w, 80, 24, END),
        lines(&[
            "",
            "  Ready when you are",
            "  No coding agent found on PATH. Install Claude Code, Codex or another and",
            "  Wormhole recognises it in any terminal, then offers its hooks once.",
            "",
            "  Things to try first",
            "    Jump to anything           ⇧⌘P",
            "    See every shortcut         ⌥⌘/",
            "    Show what changed in git   ⌃⌘1",
            "    Hide the side panels       ⌃⌘S",
            "",
            "  ❯ Start",
        ])
    );
}

#[test]
fn an_install_that_failed_is_said_above_the_footer() {
    let mut state = fixtures::state(Handover::Cursor);
    state.problem = Some("Couldn't write the hooks: permission denied".to_string());
    let w = at(Step::Agents, Welcome::new(state, true, true));
    let all = rows(&w, 80, 24, END);
    assert_eq!(all[21], "  Couldn't write the hooks: permission denied");
    assert_eq!(w.facts().problem.as_deref(), Some("Couldn't write the hooks: permission denied"));
}

// -------------------------------------------------------------------------------- the read-only tour

#[test]
fn a_tour_says_once_that_it_is_read_only_and_offers_no_install() {
    let w = Welcome::new(facts::tour_state(Path::new("/nonexistent")), false, true);
    let all = rows(&w, 80, 24, END);
    assert_eq!(all[22], "  Read-only tour: not connected to Wormhole, so nothing here changes anything.");
    assert_eq!(all.iter().filter(|row| row.contains("Read-only")).count(), 1, "one line");
    // Its agents page shows no Install button, whatever the machine has.
    let mut state = fixtures::state(Handover::Cursor);
    state.look.themes.clear();
    let tour = at(Step::Agents, Welcome::new(state, false, true));
    let text = page(&tour, 80, 24, END);
    assert!(text.iter().all(|row| !row.contains("[ Install ]")), "{text:#?}");
    assert!(text.iter().any(|row| row.contains("not installed")));
    let look = at(Step::Look, Welcome::new(facts::tour_state(Path::new("/nonexistent")), false, true));
    assert!(page(&look, 80, 24, END).iter().any(|row| row.contains("read-only")));
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
    let w = at(Step::Folder, fixtures::welcome(Handover::Cursor));
    // Past the heading and its line, each row a ROW_EVERY after the one before.
    let heading = TYPE_NAME * 18 + LINE_PAUSE;
    let note = super::render::TYPE_TEXT * 41 + LINE_PAUSE;
    let first = heading + note;
    let listed = |clock: Duration| rows(&w, 80, 24, clock)[4..8].iter().filter(|row| !row.is_empty()).count();
    assert_eq!(listed(first - Duration::from_millis(1)), 0);
    assert_eq!(listed(first), 1);
    assert_eq!(listed(first + ROW_EVERY), 2);
    assert_eq!(listed(first + ROW_EVERY * 3), 4);
    assert_eq!(listed(END), 4);
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
    assert_eq!(rows(&w, 80, 24, NOW)[4], "", "the folder list has not come in");
    w.key(Key::Left, ms(10));
    assert_eq!(w.step(), Step::Welcome);
    assert_eq!(rows(&w, 80, 24, ms(10))[4], "  ←  your terminals and what their agents are doing");
}

#[test]
fn under_reduce_motion_nothing_types() {
    let mut state = fixtures::state(Handover::Cursor);
    state.reduce_motion = true;
    let w = Welcome::new(state, true, true);
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
        let mut w = fixtures::welcome(handover);
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
        assert!(seen.contains(&Step::Look) && seen.contains(&Step::Agents), "{handover:?} {seen:?}");
    }
}

#[test]
fn the_pages_do_not_depend_on_which_handover_after_the_first() {
    let mut cursor = fixtures::welcome(Handover::Cursor);
    let mut logo = fixtures::welcome(Handover::Logo);
    for _ in 0..3 {
        cursor.key(Key::Enter, NOW);
        logo.key(Key::Enter, NOW);
        assert_eq!(rows(&cursor, 80, 24, END), rows(&logo, 80, 24, END));
    }
}

// ------------------------------------------------------------------------------------------ clicks

#[test]
fn rows_and_footer_hints_can_be_clicked() {
    let w = at(Step::Folder, fixtures::welcome(Handover::Cursor));
    let frame = render(&w, Size { cols: 80, rows: 24 }, END);
    let hit =
        |row: usize, col: usize| frame.hits.iter().find(|hit| hit.contains(row, col)).map(|hit| hit.target);
    // The four places, from row 4.
    for (line, row) in (4..8).enumerate() {
        assert_eq!(hit(row, 6), Some(Target::Row(line)));
    }
    assert_eq!(hit(8, 6), None);
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

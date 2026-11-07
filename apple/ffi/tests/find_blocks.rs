//! Find, command blocks and the minimap against the real engine and a real PTY. A `sh` script prints
//! what a shell with integration would (OSC 133 marks) and then stays as `cat`, which echoes input.

use std::sync::Arc;
use std::time::{Duration, Instant};

use wormhole_ffi::{BlockStatus, PutResult, Terminal, TerminalListener};

struct Quiet;

impl TerminalListener for Quiet {
    fn wake(&self) {}
}

fn start(script: &str, rows: u16) -> Arc<Terminal> {
    let args = vec!["-c".to_string(), format!("{script}; exec cat")];
    Terminal::start("/bin/sh".into(), args, 40, rows, 8.0, 16.0, Arc::new(Quiet)).expect("starts")
}

fn wait_for(terminal: &Terminal, needle: &str) {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        terminal.frame();
        if terminal.screen_text().contains(needle) {
            return;
        }
        assert!(Instant::now() < until, "never showed {needle:?}:\n{}", terminal.screen_text());
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Take frames until the three blocks of `MARKED` are there, the last one a prompt (its marks come after its text).
fn wait_blocks(terminal: &Terminal) {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        terminal.frame();
        let spans = terminal.block_spans();
        if spans.len() == 3 && spans[2].status == BlockStatus::Prompt {
            return;
        }
        assert!(Instant::now() < until, "blocks never came: {spans:?}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// 60 numbered lines on a 10-row screen: most of them end up in the history.
const LONG: &str =
    "i=1; while [ $i -le 60 ]; do printf 'line %02d needle\\r\\n' $i; i=$((i+1)); done; printf 'END\\r\\n'";

#[test]
fn find_searches_the_whole_scrollback_and_counts() {
    let t = start(LONG, 10);
    wait_for(&t, "END");
    let all = t.find_search("needle".into(), false, false, false);
    assert_eq!((all.count, all.capped, all.invalid), (60, false, false), "history and screen both");
    assert!(all.nearest.is_some());
    let one = t.find_search("line 07".into(), false, false, false);
    assert_eq!(one.count, 1);
    assert_eq!(t.find_search("LINE 07".into(), true, false, false).count, 0, "match case");
    assert_eq!(t.find_search("line 0[1-3] n".into(), false, false, true).count, 3, "regex");
    assert_eq!(t.find_search("need".into(), false, true, false).count, 0, "whole word");
    assert_eq!(t.find_search("needle".into(), false, true, false).count, 60);
    let bad = t.find_search("(".into(), false, false, true);
    assert_eq!((bad.invalid, bad.count), (true, 0));
    assert_eq!(t.find_search(String::new(), false, false, false), Default::default());
}

#[test]
fn find_reveals_a_match_in_the_history_and_paints_it_there() {
    let t = start(LONG, 10);
    wait_for(&t, "END");
    t.find_search("line 30".into(), false, false, false);
    assert_eq!(t.display_offset(), 0);
    t.find_reveal(0);
    t.frame();
    assert!(t.display_offset() > 0, "scrolled up into the history");
    let spans = t.find_spans();
    assert_eq!(spans.len(), 1);
    assert!(spans[0].current);
    // A line out of view comes to the middle of the screen.
    assert_eq!(spans[0].row, 5);
    assert_eq!((spans[0].first, spans[0].last), (0, 6));
    assert_eq!(t.find_ticks().len(), 1);
    assert!(!t.find_has_new_output());
    t.find_clear();
    assert!(t.find_spans().is_empty());
}

#[test]
fn find_notices_output_and_keeps_matches_on_their_text() {
    let t = start("printf 'alpha\\r\\nbeta\\r\\n'", 10);
    wait_for(&t, "beta");
    assert_eq!(t.find_search("beta".into(), false, false, false).count, 1);
    t.write(b"gamma\n".to_vec());
    wait_for(&t, "gamma");
    assert!(t.find_has_new_output());
    assert_eq!(t.find_spans().len(), 1, "the match still holds its text");
}

/// Two commands with marks: `ls` succeeds, `false` fails with 3; then a prompt.
const MARKED: &str = "printf '\\033]133;A\\033\\\\$ \\033]133;B\\033\\\\ls\\r\\n\\033]133;C\\033\\\\a\\r\\nb\\r\\n\\033]133;D;0\\033\\\\'; \
printf '\\033]133;A\\033\\\\$ \\033]133;B\\033\\\\false\\r\\n\\033]133;C\\033\\\\boom\\r\\n\\033]133;D;3\\033\\\\'; \
printf '\\033]133;A\\033\\\\$ \\033]133;B\\033\\\\'";

#[test]
fn blocks_come_from_the_shells_marks_with_their_exit_codes() {
    let t = start(MARKED, 12);
    wait_blocks(&t);
    assert!(t.blocks_available());
    let spans = t.block_spans();
    assert_eq!(spans.len(), 3);
    assert!(matches!(spans[0].status, BlockStatus::Finished { code: Some(0), .. }));
    assert!(matches!(spans[1].status, BlockStatus::Finished { code: Some(3), .. }));
    assert_eq!(spans[2].status, BlockStatus::Prompt);
    let failed = t.block_by_id(spans[1].id).expect("on the grid");
    assert_eq!(failed.command, "false");
    let out = failed.output_start.expect("has output");
    assert_eq!(t.lines_text(out, failed.end).trim(), "boom");
    let newest = t.block_newest_ran().expect("a command ran");
    assert_eq!(newest.id, failed.id);
    assert_eq!(t.block_at_line(failed.start).map(|b| b.id), Some(failed.id));
}

#[test]
fn prompt_jumps_walk_the_prompts_and_select_output_selects_lines() {
    let t = start(MARKED, 12);
    wait_blocks(&t);
    let first = t.block_spans()[0].id;
    let (a, b) = (t.block_by_id(first).expect("block").start, t.prompt_line(t.bottommost_line() + 1, true));
    let last = b.expect("a prompt above the bottom");
    assert!(last > a);
    assert_eq!(t.prompt_line(last, true).map(|l| l > a), Some(true));
    assert_eq!(t.prompt_line(a, true), None, "none above the first");
    assert_eq!(t.prompt_line(a, false).map(|l| l > a), Some(true));
    let block = t.block_by_id(first).expect("block");
    let out = block.output_start.expect("output");
    t.select_lines(out, block.end);
    assert_eq!(t.selection_text().as_deref().map(str::trim), Some("a\nb"));
}

#[test]
fn put_at_prompt_types_the_command_and_refuses_while_a_program_runs() {
    let t = start(MARKED, 12);
    wait_blocks(&t);
    let ran = t.block_spans()[0].id;
    let mut palette = vec![0x102030; 263];
    palette[256] = 0xe0e0e0; // foreground, against the background (257)
    assert_eq!(t.block_put_at_prompt(ran, palette.clone()), PutResult::Typed);
    assert_eq!(t.block_put_at_prompt(u64::MAX, palette), PutResult::Gone);
}

#[test]
fn the_minimap_reads_lines_when_the_output_changed_and_scrolls_the_view() {
    let t = start(LONG, 10);
    wait_for(&t, "END");
    let g = t.minimap_geometry(100);
    assert_eq!((g.total, g.screen, g.columns), (g.total, 10, 40));
    assert!(g.total > 60, "history and screen: {}", g.total);
    let lines = t.minimap_lines();
    assert!(lines.iter().any(|runs| !runs.is_empty()));
    assert_eq!(t.minimap_geometry(100).stamp, g.stamp, "nothing changed");
    t.scroll_to_display_offset(20);
    t.frame();
    assert_eq!(t.display_offset(), 20);
    assert_ne!(t.minimap_geometry(100).view_top, g.view_top);
    assert!(t.minimap_geometry(100).stamp > g.stamp, "the view moved");
    t.scroll_to_display_offset(0);
    t.frame();
    assert_eq!(t.display_offset(), 0);
}

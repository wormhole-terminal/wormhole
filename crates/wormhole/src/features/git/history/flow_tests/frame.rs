//! What a frame on the History tab costs, counted in allocations: the tab with a commit open, a long
//! message compact and whole, a message of thousands of paragraphs, and a log of many commits. A cost
//! per commit, per paragraph or per ref shows at once.

use gpui_kit::TestAppContext;
use wormhole_core::config::TabId;

use crate::app_state::OpenWindow;

use super::super::super::testing::{git, go, repo};
use super::{first_commit, sha_at, toggle_body, toggle_commit, two_commits};
use crate::perf::allocations::thread_counts;
use crate::window::test_harness::{draw, draw_fresh, drive, isolate_in, read, start};

// ---------------------------------------------------------------- the frame

/// Allocations the third draw of a one-terminal window on History (two commits, the newest open with its
/// details and two files) may make. Measured: 5,284 allocations (2.8 MB), the same on every run (3,856
/// before the open commit showed its details: one block of about thirty elements, three icon buttons
/// among them at about 125 each, drawn once however many commits there are, its text built when the
/// details were read and not in the frame; 3,790 before History moved
/// into its entity, 3,792 before the icons' motions: each tab's icon and each button with a motion looks
/// up the state it keeps, and a button's click handler is shared). The budget is that plus
/// about a hundredth; the tab adds a
/// row for each commit and file in view, so a cost per commit or per file per frame shows at once.
const HISTORY_FRAME_ALLOCATIONS: u64 = 5_340;

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

#[gpui_kit::test]
fn a_window_frame_on_history_stays_within_its_allocation_budget(cx: &mut TestAppContext) {
    use crate::perf::allocations::thread_counts;
    if isolate("a_window_frame_on_history_stays_within_its_allocation_budget") {
        return;
    }
    let window = start(cx, 1);
    two_commits(cx, &window, "history-frame");
    let second = first_commit(cx, &window);
    toggle_commit(cx, &window, &second.sha);
    assert_eq!(read(cx, &window, |ws| ws.right_tab()), TabId::History);
    // The first draws build what a window keeps; the third is a frame.
    draw(cx, &window);
    draw(cx, &window);
    let before = thread_counts();
    draw_fresh(cx, &window);
    let drawn = thread_counts().since(before);
    eprintln!("window frame on History: {} allocations, {} bytes", drawn.allocations, drawn.bytes);
    assert!(
        drawn.allocations <= HISTORY_FRAME_ALLOCATIONS,
        "a window frame on History made {} allocations (budget {HISTORY_FRAME_ALLOCATIONS})",
        drawn.allocations
    );
}

/// The allocations of the third draw of the window: the first ones build what a window keeps.
fn frame_allocations(cx: &mut TestAppContext, window: &OpenWindow) -> u64 {
    draw(cx, window);
    draw(cx, window);
    let before = thread_counts();
    draw_fresh(cx, window);
    thread_counts().since(before).allocations
}

/// A repository with one commit whose message is `message`, the terminal in it on the History tab and the
/// commit open, its details read.
fn open_commit_with(cx: &mut TestAppContext, window: &OpenWindow, name: &str, message: &str) {
    let root = repo(name, &[("a.txt", "one\n")]);
    // A commit with no files: the rows under the details are one short line, so whatever the message
    // takes of the window, the same rows are drawn and the counts differ by the details alone.
    let file = root.join("..").join(format!("{name}-message.txt"));
    std::fs::write(&file, message).expect("write the message");
    git(&root, &["commit", "-q", "--allow-empty", "-F", file.to_str().expect("a utf-8 path")]);
    drive(cx, window, |ws, _, cx| {
        go(ws, &root, cx);
        ws.show_tab(TabId::History, cx);
    });
    cx.run_until_parked();
    let sha = sha_at(cx, window, 0);
    toggle_commit(cx, window, &sha);
}

/// Allocations a frame makes with a commit open whose message is a few long paragraphs, compact and
/// whole (measured by [`a_long_message_costs_a_frame_less_while_it_is_compact`]). Whole is not the dearer
/// of the two in a window this small: text below the window's edge is laid out but not painted, and it is
/// the painting that allocates. What the cap bounds is the layout, and that is what the next two tests
/// pin with messages that would otherwise be thousands of elements.
const COMPACT_FRAME_ALLOCATIONS: u64 = 4_680;
const WHOLE_FRAME_ALLOCATIONS: u64 = 4_490;
/// A squash message of six thousand lines whole, and fifteen thousand one-letter paragraphs whole. (The
/// squash frame made 4,634 allocations on the commit before the Files tab and makes 4,675 with it: the
/// tab is 41 more in the bar, which a whole frame builds. The budget is that plus a few.)
const SQUASH_FRAME_ALLOCATIONS: u64 = 4_680;
const PARAGRAPHS_FRAME_ALLOCATIONS: u64 = 4_580;

#[gpui_kit::test]
fn a_long_message_costs_a_frame_less_while_it_is_compact(cx: &mut TestAppContext) {
    if isolate("a_long_message_costs_a_frame_less_while_it_is_compact") {
        return;
    }
    let window = start(cx, 1);
    let paragraph = "A long paragraph that says why the change was made, in the way people write them: several \
                     sentences, each a little too long for the line it is on, so the message wraps.";
    let body: Vec<String> = (1..=6).map(|n| format!("Paragraph {n}. {paragraph} {paragraph}")).collect();
    open_commit_with(
        cx,
        &window,
        "long-message",
        &format!("feat: a message with a long body\n\n{}", body.join("\n\n")),
    );
    let compact = frame_allocations(cx, &window);
    toggle_body(cx, &window);
    let whole = frame_allocations(cx, &window);
    eprintln!("a frame with a long message: {compact} allocations compact, {whole} whole");
    assert!(compact <= COMPACT_FRAME_ALLOCATIONS, "compact: {compact} (budget {COMPACT_FRAME_ALLOCATIONS})");
    assert!(whole <= WHOLE_FRAME_ALLOCATIONS, "whole: {whole} (budget {WHOLE_FRAME_ALLOCATIONS})");
}

/// A squash merge's message, which lists every commit it folded in (here six thousand lines), shown
/// whole: what is drawn is bounded, so it costs no more than a long message does.
#[gpui_kit::test]
fn a_squash_merge_message_costs_a_frame_no_more_than_a_long_one(cx: &mut TestAppContext) {
    if isolate("a_squash_merge_message_costs_a_frame_no_more_than_a_long_one") {
        return;
    }
    let window = start(cx, 1);
    let squash: String = (0..6_000).map(|n| format!("* commit {n}: a change that was folded in\n")).collect();
    open_commit_with(cx, &window, "squash", &format!("feat: squash\n\n{squash}"));
    toggle_body(cx, &window);
    let squashed = frame_allocations(cx, &window);
    eprintln!("a frame with a squash message whole: {squashed}");
    assert!(
        squashed <= SQUASH_FRAME_ALLOCATIONS,
        "a squash message whole: {squashed} (budget {SQUASH_FRAME_ALLOCATIONS})"
    );
}

/// A hostile message of fifteen thousand one-letter paragraphs, whole: not an element each.
#[gpui_kit::test]
fn a_message_of_thousands_of_paragraphs_costs_a_frame_no_more_than_a_long_one(cx: &mut TestAppContext) {
    if isolate("a_message_of_thousands_of_paragraphs_costs_a_frame_no_more_than_a_long_one") {
        return;
    }
    let window = start(cx, 1);
    open_commit_with(cx, &window, "hostile", &format!("x\n\n{}", "y\n\n".repeat(15_000)));
    toggle_body(cx, &window);
    let paragraphs = frame_allocations(cx, &window);
    eprintln!("a frame with 15,000 paragraphs whole: {paragraphs}");
    assert!(
        paragraphs <= PARAGRAPHS_FRAME_ALLOCATIONS,
        "15,000 paragraphs whole: {paragraphs} (budget {PARAGRAPHS_FRAME_ALLOCATIONS})"
    );
}

/// A log of a hundred and fifty commits, the newest open: the list draws the rows in view, so the frame
/// costs what the rows that fit do, not what the log has.
#[gpui_kit::test]
fn a_log_of_many_commits_stays_within_the_frame_budget(cx: &mut TestAppContext) {
    if isolate("a_log_of_many_commits_stays_within_the_frame_budget") {
        return;
    }
    let window = start(cx, 1);
    let root = repo("many-commits", &[("a.txt", "one\n")]);
    for n in 0..150 {
        git(
            &root,
            &[
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                &format!("commit number {n}: with a subject of some length to it"),
            ],
        );
    }
    drive(cx, &window, |ws, _, cx| {
        go(ws, &root, cx);
        ws.show_tab(TabId::History, cx);
    });
    cx.run_until_parked();
    let sha = sha_at(cx, &window, 0);
    toggle_commit(cx, &window, &sha);
    let many = frame_allocations(cx, &window);
    eprintln!("a frame with 151 commits, the newest open: {many}");
    assert!(
        many <= MANY_COMMITS_FRAME_ALLOCATIONS,
        "151 commits: {many} (budget {MANY_COMMITS_FRAME_ALLOCATIONS})"
    );
}

const MANY_COMMITS_FRAME_ALLOCATIONS: u64 = 6_440;

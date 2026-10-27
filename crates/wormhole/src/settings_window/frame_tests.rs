//! What a Settings frame costs in allocations, page by page, with the words of the window in the
//! catalogs. A word without variables is an array index and costs none; a sentence with variables is
//! formatted by Fluent (a few allocations more than a `format!`, five for one with a number), and a page
//! that does so for every row of every frame would show here.
//!
//! [`BUDGET`] is what a third draw of each page allocates today plus [`SLACK`], a fixed margin for small
//! drift. The reference is what a third draw allocated while the words were still written in Rust
//! (measured on the commit before the Settings window's words moved): Profile 9,847, Appearance 14,383,
//! Fonts 7,247, Terminal 12,521, Sessions 13,750, Git 7,745, Agents 27,044, Notifications 10,353,
//! MenuBar 7,852, Updates 6,621, Advanced 7,318, Projects 12,190, FeatureFlags 6,742, search 13,259.
//! The pages with sentences made a few more (2 to 14 of about 10,000); the search made 14% fewer, since
//! its words are lower-cased once per language and no longer for every keystroke. The search's budget
//! rose by 49 when a link's hover fill became a layer of its own, out of the layout (so the link's text
//! lines up with the edge it sits at); the results show several links. Every page rose by 2 to 3 per row
//! when a row's hover fill became such a layer too, reaching past the row's text on both sides. Pages
//! without a preview of their own (Profile, Terminal, …) fell by about 3,000 when they lost the
//! window's miniature; Appearance rose by about 300 with each theme card's hide button and menu.
//! Every page rose by about 100 when the nav got a row more, Motion; its own page, behind a feature flag,
//! draws a select under each of its four choices and a preview (a view of its own, about 450) under each, with its caption (6,250).
//! Liquid Glass moved from Appearance (one row fewer) to Feature flags (one row more); Feature flags rose by
//! 392 (4,333 to 4,725, measured with the row left out and in) with one more, Tear off a terminal: a switch,
//! its name, its line of words and its dot. It rose by 358 more (4,725 to 5,083) with Live resize, the same row
//! again, and by about 392 more with Notice chip.
//! Appearance and a search (which show the window's preview) rose by about 770 when the preview
//! became the window's own parts at full size instead of a drawing of them.
//! Updates rose by about 380 with its one more row, Count this Mac anonymously, and by about 150 with
//! the note under it: what a check sends, and in a fresh data dir when counting starts; then by about
//! 1,700 when that became a Privacy group of its own (its heading, why, the switch, the first count's
//! banner and three lines of what a check sends).
//! Appearance rose by about 370 with the Branch colour row (a segmented control and its quiet line).
//! Every page reads the settings as JSON each frame (`Values`), twice, so its two new keys cost every page
//! about 10; the colour pickers' check, which no longer builds hex strings, gave back 8. The budgets
//! were set again from that frame, which also took in a drift of up to 20 on Terminal and Updates.
//! Every page rose by 19 when the settings gained the title row's list of parts (one more heap list in
//! each copy of them a frame makes; Advanced, which reads them twice as often, by 38); Appearance by
//! about 585 more when its four switches for the title row's extras became that list, a row with a grip
//! and a switch per part; and the search fell by 512 with three entries fewer.
//! Appearance rose by about 360 and Updates by 22 with the Animate icons row and the sidebar's strip
//! fills; the icon motions of the buttons in the preview add almost none (measured with them out of the
//! frame: the same).
//! Every page and the search rose by about 65 when the nav gained its Import row (the row's icon,
//! label and hover layer: exactly 65 on each page). Terminal and Updates stand 12 and 20 above the numbers
//! this budget had before, but that is not the Import row: the commit before it already drew 9,600 and
//! 5,478 there, inside [`SLACK`] of the old 9,588 and 5,458, and the same goes for 2 on Appearance, Feature
//! flags and the search. The Import page is measured over the visual checks' fixture home (three terminals,
//! Ghostty's card open with its 13 rows and what doesn't carry over, and the comparison), not over the
//! "Looking for terminals" line a fresh page shows.
//! On main's frame (the strip fills, the icon motions and the Motion page) it measured 53,067.
//! Agents fell from 26,827 to 9,011 when the tests stopped seeing the agents installed on the Mac that
//! runs them (`WORMHOLE_AGENT_PATH` empty): an installed agent's row draws its hooks and logins, so the
//! page cost what that Mac had installed. Pi's row and the logos of Codex, Amp, goose and Kiro came at
//! the same time and cost nothing measurable (8,662 before them, 8,649 after, measured before the
//! command line's row: a logo is cheaper than the two letters it replaces).
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.

use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{AnyWindowHandle, TestAppContext};
use wormhole_core::config::settings_index::Query;

use super::sections::{MotionPreview, MotionScene as Scene};
use super::{Section, current_window, open_settings};
use crate::perf::allocations::thread_counts;
use crate::window::test_harness::{isolate_in, start};

/// Allocations a frame may make over what the pages made when this budget was set.
const SLACK: u64 = 20;

/// Allocations a third draw of each page made when the budget was set (the first two build what a window
/// keeps), and of the results page of a search for a word found in several pages; a frame may make
/// [`SLACK`] more.
const BUDGET: [(&str, u64); 16] = [
    ("Profile", 6761),
    ("Appearance", 16709),
    ("Fonts", 7459),
    ("Motion", 6340),
    ("Terminal", 9771),
    // Three choices and a switch more (what the Changed files count is of, plain terminals, files changed
    // at once, remember after restart): about 1,400, each a segmented control or a switch with its line.
    ("Sessions", 15115),
    ("Git", 4567),
    // A row more (Let the command line read terminals), and the Wormhole skill group: its heading and
    // blurb, the line that no agent was found (the tests see none), and the MCP command with its Copy
    // button; 668 more. The Agent access group (a heading and blurb, the mode's three segments and the line
    // that nothing has asked yet) came with the Automation flag on in place of the two switch rows, and the
    // page measured 9,501: under this, which stays.
    ("Agents", 9679),
    ("Import", 53067),
    ("Notifications", 7882),
    ("MenuBar", 6195),
    ("Updates", 5649),
    // The four keys of the Changed files choices: Advanced reads the settings as JSON twice a frame where
    // the other pages do once, so they cost it twice what they cost the others (36, not 18).
    ("Advanced", 4185),
    ("Projects", 9037),
    // The switches for Automation, Tear off, Live resize and Notice chip are four rows more; the
    // `agent_access` key, which every page reads as JSON each frame, a few more.
    ("FeatureFlags", 5985),
    // The same row is one more for the search to draw; the four keys of the Changed files choices, which
    // every page reads as JSON each frame, are about 14 more.
    ("search", 12250),
];

/// What a scan finds in the visual checks' fixture home (Ghostty, iTerm2 and Alacritty settings), with the
/// fonts asked of the system as the page's own scan does: never this Mac's own home.
fn fixture_scan() -> Vec<wormhole_core::import::Found> {
    let home =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/visual/fixtures/import/home");
    let mut env = wormhole_core::import::Env::new(home);
    env.login_shell = "/bin/zsh".into();
    env.system_shells = ["/bin/zsh", "/bin/bash", "/bin/sh"].map(String::from).to_vec();
    let mut found = wormhole_core::import::scan(&env);
    crate::settings::import::resolve_fonts(&mut found);
    assert!(found.len() >= 3, "the fixture home has three terminals");
    found
}

fn draw(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    cx.update(|cx| handle.update(cx, |_, window, cx| window.render_frame(cx)))
        .expect("the settings window is open");
}

#[gpui_kit::test]
fn a_settings_frame_stays_within_its_allocation_budget(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_settings_frame_stays_within_its_allocation_budget") {
        return;
    }
    start(cx, 1);
    cx.update(|cx| open_settings(Some(Section::Profile), cx));
    cx.run_until_parked();
    let (handle, page) = cx.update(|cx| current_window(cx)).expect("the settings window opened");
    let mut measured = Vec::new();
    for section in Section::ALL {
        cx.update(|cx| page.update(cx, |page, cx| page.show_section(section, cx)));
        cx.run_until_parked();
        if section == Section::Import {
            // The page's own scan has run over the scratch home and found nothing; measure the page
            // that has something to show: the cards, the open one's rows, the comparison.
            let found = fixture_scan();
            cx.update(|cx| page.update(cx, |page, cx| page.set_import(found, cx)));
            cx.run_until_parked();
        }
        draw(cx, handle);
        draw(cx, handle);
        let before = thread_counts();
        draw(cx, handle);
        measured.push((format!("{section:?}"), thread_counts().since(before)));
    }
    // A search: the results of a word found in several pages, over the pages' rows.
    cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| {
                page.update(cx, |page, cx| {
                    page.search.update(cx, |search, cx| search.set_value("line".to_string(), window, cx));
                    page.show_results = !Query::new("line").is_empty();
                    cx.notify();
                })
            })
            .expect("the window is open");
    });
    cx.run_until_parked();
    draw(cx, handle);
    draw(cx, handle);
    let before = thread_counts();
    draw(cx, handle);
    measured.push(("search".to_string(), thread_counts().since(before)));

    let table: Vec<String> = measured
        .iter()
        .map(|(name, drawn)| format!("{name}: {} allocations, {} bytes", drawn.allocations, drawn.bytes))
        .collect();
    let over: Vec<String> = measured
        .iter()
        .zip(BUDGET)
        .filter(|((_, drawn), (_, budget))| drawn.allocations > budget + SLACK)
        .map(|((name, drawn), (_, budget))| {
            format!("{name}: {} (budget {})", drawn.allocations, budget + SLACK)
        })
        .collect();
    assert!(over.is_empty(), "frames over their budget:\n{}\nall:\n{}", over.join("\n"), table.join("\n"));
}

/// Allocations a third draw of each Motion preview makes on its own (a window holding nothing else),
/// when the budget was set: what a tick of one costs, since a preview builds only what shows at its
/// moment of the loop. The four are redrawn by their own clocks, a few times a second each.
const PREVIEW_BUDGET: [(Scene, u64); 4] =
    [(Scene::Insert, 487), (Scene::Fold, 543), (Scene::Status, 437), (Scene::Column, 476)];

// A tick draws the window like any other draw: GPUI renders the root view on every draw and marks the
// ancestors of the view that asked for it dirty, so the page is built again with the previews. What the
// previews add is their own work, which is what the first half holds down; the second half holds the
// tick to the cost of a draw at rest, so nothing grows with the clock.
#[gpui_kit::test]
fn a_motion_preview_tick_costs_hundreds_of_allocations_and_the_page_no_more_than_at_rest(
    cx: &mut TestAppContext,
) {
    if isolate_in(
        module_path!(),
        "a_motion_preview_tick_costs_hundreds_of_allocations_and_the_page_no_more_than_at_rest",
    ) {
        return;
    }
    start(cx, 1);
    // Each preview alone: its draw is what its clock asks for.
    let mut table = Vec::new();
    for (scene, budget) in PREVIEW_BUDGET {
        let window = cx.add_window(|_, cx| MotionPreview::new(scene, cx));
        let handle: AnyWindowHandle = window.into();
        draw(cx, handle);
        draw(cx, handle);
        let before = thread_counts();
        draw(cx, handle);
        let made = thread_counts().since(before).allocations;
        table.push((scene, made, budget));
    }
    let over: Vec<String> = table
        .iter()
        .filter(|(_, made, budget)| *made > budget + SLACK || *made >= 1500)
        .map(|(scene, made, budget)| format!("{scene:?}: {made} (budget {})", budget + SLACK))
        .collect();
    assert!(over.is_empty(), "previews over their budget:\n{}\nall: {table:?}", over.join("\n"));
    // The page: a tick draws what a draw at rest does, plus what the previews' own redraw is.
    cx.update(|cx| open_settings(Some(Section::Motion), cx));
    cx.run_until_parked();
    let (handle, _) = cx.update(|cx| current_window(cx)).expect("the settings window opened");
    draw(cx, handle);
    draw(cx, handle);
    let before = thread_counts();
    draw(cx, handle);
    let rest = thread_counts().since(before).allocations;
    cx.executor().advance_clock(std::time::Duration::from_millis(40));
    cx.run_until_parked();
    let before = thread_counts();
    draw(cx, handle);
    let tick = thread_counts().since(before).allocations;
    assert!(tick <= rest + SLACK, "a tick drew {tick}, a draw at rest {rest}");
}

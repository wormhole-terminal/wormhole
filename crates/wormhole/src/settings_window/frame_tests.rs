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
//! Liquid Glass moved from Appearance (one row fewer) to Feature flags (one row more), and back as a beta
//! (on macOS 26 and later only: [`GLASS_ROW`] is what its row adds there); Feature flags rose by
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
//! Feature flags rose by about 380 when the page gained a row for the Folder picker flag (6,874 measured).
//! On main's frame (the strip fills, the icon motions and the Motion page) it measured 53,067.
//! Agents fell from 26,827 to 9,011 when the tests stopped seeing the agents installed on the Mac that
//! runs them (`WORMHOLE_AGENT_PATH` empty): an installed agent's row draws its hooks and logins, so the
//! page cost what that Mac had installed. Pi's row and the logos of Codex, Amp, goose and Kiro came at
//! the same time and cost nothing measurable (8,662 before them, 8,649 after, measured before the
//! command line's row: a logo is cheaper than the two letters it replaces).
//! Rows lost their line of description for an (i) after the label, a tab stop with its card built only
//! while it is open (`style::info`). That costs a row its element, its focus, its click and its ring
//! (measured on Terminal with the (i)s left out and in: 9,895 and 10,333), so Fonts rose from 7,560 to
//! 7,634, Terminal from 10,228 to 10,333, Sessions from 15,198 to 15,454, Notifications from 7,983 to
//! 8,007 and MenuBar from 6,296 to 6,344 (the other pages stayed under theirs). With the Kit's popover and
//! tooltip on every row the same pages made 8,263, 11,773, 16,714, 8,907 and 6,884.
//! Each (i) then took a press listener of its own, so that a press which closes one card settles, at that
//! press, whether the click after it is the (i)'s own (a dismissal left over by an earlier press then
//! matches nothing). That is about 3 allocations per (i): Fonts rose from 7,634 to 7,655, Terminal from
//! 10,333 to 10,381, Sessions from 15,454 to 15,496 and Notifications from 8,007 to 8,037 (measured).
//! A row with a config key then took its menu (a right click, ⌃↩ and VoiceOver's Show menu: three listeners
//! on the row, each a closure and a registration GPUI makes as the frame paints) and a click on its key
//! (an element with an id of its own, so its state and click too): 19 allocations per such row (measured on
//! Terminal's 17 such rows: 10,381 to 10,705). The
//! menu, its items and the JSON are built only once it is open, and the key's words are not copied.
//! Appearance rose from 17,241 to 17,367, Fonts from 7,655 to 7,750, Motion from 6,441 to 6,462, Terminal
//! from 10,381 to 10,705, Sessions from 15,496 to 15,764, Git from 4,668 to 4,746, Notifications from
//! 8,037 to 8,246, MenuBar from 6,344 to 6,476 and Updates from 5,750 to 5,786.
//! The nav's order changed (the budgets are listed in it) and its rows became keyboard stops: each row is a
//! button with a name, a tab stop and a focus ring, about 13 allocations, so every page rose by 189 (Profile,
//! which lists no sub-rows, measured 6,862 before and 7,051 after) and by about 136 for each sub-row under the
//! open page, which is one more line of text to shape plus that (Terminal's four: 10,894 to 11,437, measured
//! with the sub-rows left out and in). Terminal 10,705 to 11,437, Appearance 17,367 to 18,014, Fonts 7,750 to
//! 8,292, Sessions 15,764 to 16,229, Git 4,746 to 5,113, Notifications 8,246 to 8,795, Agents 9,679 to 10,466,
//! MenuBar 6,476 to 6,839, Updates 5,786 to 6,140, Advanced 4,687 to 5,136, Motion 6,462 to 6,816, Projects
//! 9,138 to 9,283, Extensions 2,686 to 2,845, Feature flags 6,912 to 7,074 and the search 12,250 to 12,274
//! (no sub-rows while searching). Import rose from 53,166 to 53,549.
//! Every segmented control became a menu control (`select::menu`): one focusable element around the
//! select's face, its options, closures and card built only while open, in place of a track and a segment
//! per option. A row with two options rose by about 10 (Updates 6,140 to 6,160, Git 5,113 to 5,127 with
//! three of them and the tab width stepper's two buttons gone, MenuBar 6,839 to 6,852) and one with three
//! or more fell; Sessions, with six of them, rose from 16,229 to 16,284 (measured). Notifications fell
//! from 8,795 to 8,645 with the stepper beside the command threshold's switch, which is one menu now.
//! Terminal fell from 11,437 to 11,421, Appearance from 18,014 to 17,988, Agents from 10,466 to 10,460
//! and the search from 12,274 to 12,250.
//!
//! The menu controls then took the width of their value instead of a fixed one, which makes GPUI measure
//! the text of every closed menu (about 35 allocations each, measured as Git's three: 5,127 to 5,231), and
//! the four selects that were a `SearchSelect` (Scrollback, Open folders in, Default shell, Icon theme, and
//! Motion's four) became menu controls too, which build nothing until open. Sessions rose from 16,284 to
//! 16,440, Git to 5,231, Notifications from 8,645 to 8,749, MenuBar from 6,852 to 6,904, Updates from 6,160
//! to 6,186, Agents from 10,460 to 10,486 and Appearance from 17,988 to 18,033 (a menu more: Icon theme,
//! less its popover and trigger); Terminal fell from 11,421 to 11,340, Motion from 6,816 to 6,787.
//!
//!
//! Step 7a of the Settings redesign: the primary and branch colours are one colour control each (a swatch and
//! a name, its card built only while open) in place of a switch with a row of swatches under it and a menu with
//! another: Appearance rose from 18,117 to 18,189. The nav counts the settings that differ for the Changed
//! page's badge (one more read of the settings as JSON a frame, about 340), which paid for itself: the
//! defaults they are compared with are made once now and not by every read (about 270 each), so the
//! other pages stand within 10 of their old budgets, Sessions 89, Advanced 157 and Import 1,491 under (they read the
//! settings more often); Profile rose by 65, the one page the new read costs more
//! than the cache gives back. The Changed page, 5,541 over three changed settings, is new.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.

use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{AnyWindowHandle, TestAppContext};
use wormhole_core::config::settings_index::Query;

use super::sections::{MotionPreview, MotionScene as Scene};
use super::{Section, current_window, open_settings};
use crate::perf::allocations::thread_counts;
use crate::settings::update_settings;
use crate::window::test_harness::{isolate_in, start};

/// Allocations a frame may make over what the pages made when this budget was set.
const SLACK: u64 = 20;

/// Allocations a third draw of each page made when the budget was set (the first two build what a window
/// keeps), and of the results page of a search for a word found in several pages; a frame may make
/// [`SLACK`] more.
const BUDGET: [(&str, u64); 18] = [
    ("Profile", 7116),
    // The Changed page is measured with three settings changed (Tab width, Wrap long lines, Blur), each in the
    // row of its own page with its default under its name: 5,541.
    ("Changed", 5541),
    // With the Worktrees group's switch for cloning build folders (356). Then the preview card of two split
    // terminals (its label, two panes with their headers, five runs of text each and a minimap's bars):
    // 12,441 measured, 1,150 more.
    ("Terminal", 12435),
    // With the Liquid Glass row (16,810 before it came back).
    // The preview moved from the right-hand column into a card at the top of the page: 18,117 measured, 84
    // more (the card's own label, group and caption; the window it draws is the same).
    // With the Borders switch (a row, its (i) and its key line): 18,615 measured, 426 more.
    ("Appearance", 18615),
    // The specimen is a card on the page now, not in the column: 8,266 measured, 26 fewer.
    ("Fonts", 8260),
    // Three choices and a switch more (what the Changed files count is of, plain terminals, files changed
    // at once, remember after restart): about 1,400, each a segmented control or a switch with its line.
    // The list preview is a card at the top of the page, with the column's second window gone: 13,612
    // measured, 2,828 fewer than the page and the column made together.
    ("Sessions", 13523),
    // The preview card of the Changes panel and a diff: the panel's own file rows (four, with the folder
    // rows of the tree and the untracked head) and the diff view's own rows (a hunk head and six lines, each
    // a row with two gutters, a marker and its code): 8,367 measured, 3,136 more, about 340 for each of the
    // nine rows.
    ("Git", 8361),
    // The banners are a card on the page: 8,793 measured, 44 more than in the column (the card's label,
    // group and caption).
    ("Notifications", 8787),
    // A row more (Let the command line read terminals), and the Wormhole skill group: its heading and
    // blurb, the line that no agent was found (the tests see none), and the MCP command with its Copy
    // button; 668 more. The Agent access group (a heading and blurb, the mode's three segments and the line
    // that nothing has asked yet) came with the Automation flag on in place of the two switch rows, and the
    // page measured 9,501: under this, which stays.
    // The line that no agent was found became the empty list (a card with a title, the sentence and an
    // icon): 11,013 measured, 527 more, all of it the second text block's shaping and the card.
    // The agents list is a card on the page: 11,076 measured, 63 more than in the column.
    ("Agents", 11070),
    // The menu bar and the quick panel are a card on the page: 6,962 measured, 58 more than in the column.
    ("MenuBar", 6956),
    // Fell from 53,549 to 50,188 (measured) when the group labels, notices and buttons became one
    // component each; Terminal, Updates, Motion, Projects and Extensions fell by 3 to 589 with them.
    ("Import", 48697),
    // Fell by 37 when the status became a tile row.
    ("Updates", 5552),
    // The four keys of the Changed files choices: Advanced reads the settings as JSON twice a frame where
    // the other pages do once, so they cost it twice what they cost the others (36, not 18).
    // With the New programs group: the Developer Tools row, its description and its button (417).
    ("Advanced", 4979),
    ("Motion", 6779),
    // The Trusted projects list, empty in the tests, says so in the empty list (title, hint, card):
    // 10,264 measured, 983 more than the one line it was.
    ("Projects", 10259),
    // Measured with the nav's Motion and Import rows, which came after the page.
    ("Extensions", 2832),
    // The switches for Automation, Tear off, Live resize, Notice chip and Probes are five rows more; the
    // `agent_access` key, which every page reads as JSON each frame, a few more.
    // The Files flag is one switch row more: 6,478 before it, 6,875 with it (397, as each earlier row cost
    // about 380); the Folder picker flag one more again (about 385). Liquid Glass went back to Appearance:
    // 6,912 measured, a row fewer.
    // The Restore after restart flag is one switch row more again: 7,271 measured (about 340 a row).
    ("FeatureFlags", 7290),
    // The same row is one more for the search to draw; the four keys of the Changed files choices, which
    // every page reads as JSON each frame, are about 14 more.
    // The results no longer show the window's miniature beside them (it was in the column, whatever was
    // found): 8,042 measured, 4,221 fewer.
    // The search finds the Borders row too, as one more result: 8,456 measured, 420 more.
    ("search", 8456),
];

/// What Appearance's Liquid Glass row adds, a switch with its line and Beta chip: it is on the page on
/// macOS 26 and later only, so a Mac before that draws the page without it.
const GLASS_ROW: u64 = 431;

/// `budget` of the page `name` on this Mac.
fn budget_here(name: &str, budget: u64) -> u64 {
    let without_glass = name == "Appearance" && !wormhole_core::platform::system::has_liquid_glass();
    budget - if without_glass { GLASS_ROW } else { 0 }
}

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
        // The Changed page is measured with three settings changed (and the nav listing it): over the
        // defaults it is one line.
        if section == Section::Changed {
            cx.update(|cx| {
                update_settings(cx, |s| {
                    s.tab_width = 8;
                    s.diff_wrap = true;
                    s.blur = !crate::settings::Settings::default().blur;
                })
            });
        }
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
        if section == Section::Changed {
            cx.update(|cx| update_settings(cx, |s| *s = crate::settings::Settings::default()));
        }
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
        .map(|((name, drawn), (_, budget))| (name, drawn, budget_here(name, budget) + SLACK))
        .filter(|(_, drawn, budget)| drawn.allocations > *budget)
        .map(|(name, drawn, budget)| format!("{name}: {} (budget {budget})", drawn.allocations))
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

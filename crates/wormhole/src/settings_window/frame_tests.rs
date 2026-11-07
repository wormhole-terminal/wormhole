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
//! Ghostty's card open with its 13 rows and what doesn't carry over), not over the
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
//! Every page rose by 81 when the nav gained its Changed row (the row itself, and the count behind its badge,
//! which reads the settings as JSON once more), the Changed page being new at 6,486 with the five settings of
//! [`change`] listed. `Values` keeps the defaults it compares with from the first read on (they never change),
//! which every frame used to serialise again, so the search fell by 70 and Advanced, which reads them most,
//! by 57.
//! The Fonts page gained the Settings text size row, a select like the Appearance page's icon theme (the
//! trigger, the search select's state and the row's own listeners): Fonts rose from 7,366 to 7,766.
//!
//! Settings' groups became one rounded card each: the rows' lines became the dividers the group draws
//! between them, and the lists some pages built by hand went into the same group. Every page was measured
//! again (the card, a divider element between each two rows and a gap between a head and its card cost a
//! few allocations a group; a row's hairline cost none).
//!
//! Native short-list menus share the registry and build their items only when opened. The budget below
//! includes the sidebar's keyboard navigation, control focus and accessibility, and the shared selection
//! fades. The Appearance shelf's keyboard navigation keeps the per-card cache: changing focus does not
//! redraw every theme.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.

use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{AnyWindowHandle, TestAppContext};
use wormhole_core::config::settings_index::Query;

use super::sections::{MotionPreview, MotionScene as Scene};
use super::{Section, current_window, open_settings};
use crate::perf::allocations::thread_counts;
use crate::settings::{settings, update_settings};
use crate::window::test_harness::{isolate_in, start};

/// Allocations a frame may make over what the pages made when this budget was set.
const SLACK: u64 = 20;

/// Allocations a third draw of each page made when the budget was set (the first two build what a window
/// keeps), and of the results page of a search for a word found in several pages; a frame may make
/// [`SLACK`] more.
// Measured with Show config keys off (the default): a page does not build its rows' key lines then, which
// is why these are lower than they were when the lines were always built.
//
// The pages with a preview were measured again when the strip became one fixed height with nothing in it that
// scrolls: Appearance's window is a small whole one (a sidebar with one terminal, four lines of terminal;
// 2,700 fewer allocations, and the search, which shows it, 2,700 fewer), the Terminals list is one group's
// card without its header (2,400 fewer), and the diff sample is two rows shorter (Changes & diffs 450 fewer);
// the specimen's facts are one string (Fonts 80 more). The nav's divider, an element of its own, is 3 on
// every page.
// The flat text fields (an id for the hover, the round clear button) and the nav's search moving onto them:
// Profile, which has four fields of its own, rose by 57, and Terminal, Agents, MenuBar, Hotkey and Updates
// by 21 to 29 each (the Hotkey page's recorder is flat now too).
const BUDGET: [(&str, u64); 20] = [
    ("Profile", 7540),
    // Measured with [`change`]'s settings differing from their defaults, so the page has rows: five settings
    // on four pages, each a row of its own page with a dot, a reset arrow and a menu.
    ("Changed", 6854),
    // With the Liquid Glass row (16,810 before it came back), and the Borders switch: a row, its (i) and its
    // key line, 425 more.
    // The window miniature fills the stage (the sidebar keeps its width, the terminal takes the rest): 15,438
    // measured, about 490 more than at its fixed size.
    ("Appearance", 15909),
    // With the flat controls: a stepper is two halves, each a box with the Kit's button in it, and the
    // slider is built from the Kit's parts; a page with either costs a few hundred allocations more.
    ("Fonts", 8342),
    // Four scenes at once in one strip, each its stage and caption (5,991 measured).
    ("Motion", 6114),
    // With the Worktrees group's switch for cloning build folders (356), and the Inline images group's one
    // select, Image memory per terminal (543: a row with its label, help line and a native menu's state).
    // The Never hide lines switch is one row more in Behaviour: 11,349 measured (about 328 a row).
    ("Terminal", 11329),
    // Three choices and a switch more (what the Changed files count is of, plain terminals, files changed
    // at once, remember after restart): about 1,400, each a segmented control or a switch with its line.
    // The list in all three densities side by side, each with a group card of three rows and two more cards, in
    // place of one list with one row (19,454 measured).
    ("Sessions", 19983),
    ("Git", 6127),
    // A row more (Let the command line read terminals), and the Wormhole skill group: its heading and
    // blurb, the line that no agent was found (the tests see none), and the MCP command with its Copy
    // button; 668 more. The Agent access group (a heading and blurb, the mode's three segments and the line
    // that nothing has asked yet) came with the Automation flag on in place of the two switch rows, and the
    // page measured 9,501: under this, which stays.
    // Six cards in two rows in place of three (11,684 measured).
    ("Agents", 12069),
    // Fell from 49,182 when the comparison left the page: the open card and its rows are what is drawn.
    ("Import", 23391),
    // Four banners in two rows in place of one (8,469 measured).
    ("Notifications", 8913),
    // The page was split into MenuBar and Hotkey, each with a preview that is a view of its own (a menu bar
    // twice, and the shortcut's keys with the quick panel or the window sketch); both measured at the split.
    // Every other page rose by 68 with the nav's one row more, Hotkey, and was measured again.
    // Four stretches of the menu bar in place of two (3,973 measured).
    ("MenuBar", 4339),
    // The strip's flat stage became one element round every preview, in place of the frame each drew: Hotkey
    // measured 3,973 (74 more than before; where the 74 came from was not isolated: the preview is a view of
    // its own, drawn in the stage). With three results in the quick panel in place of one: 4,290.
    ("Hotkey", 4624),
    ("Updates", 5063),
    // The four keys of the Changed files choices: Advanced reads the settings as JSON twice a frame where
    // the other pages do once, so they cost it twice what they cost the others (36, not 18).
    // With the New programs group: the Developer Tools row, its description and its button (417).
    // The Show config keys switch is a row more, 4,941 measured; with the keys hidden by default every
    // other page fell (their key lines are not built).
    // The flat buttons (`style::Button`) read the window's colours as they are drawn: 3 more, 5,337 measured.
    ("Advanced", 5626),
    ("Projects", 11042),
    // Measured with the nav's Motion and Import rows, which came after the page.
    ("Extensions", 3169),
    // The switches for Automation, Tear off, Live resize, Notice chip and Probes are five rows more; the
    // `agent_access` key, which every page reads as JSON each frame, a few more.
    // The Files flag is one switch row more: 6,478 before it, 6,875 with it (397, as each earlier row cost
    // about 380); the Folder picker flag one more again (about 385). Liquid Glass went back to Appearance:
    // 6,912 measured, a row fewer.
    // The Restore after restart flag is one switch row more again: 7,271 measured (about 340 a row).
    // The File icon themes flag is one switch row more again: 7,577 measured (about 305 a row).
    // The Fold output flag is one more: 7,803 measured (about 297 a row).
    ("FeatureFlags", 7786),
    // Settings → Components, a development tool: every specimen of the controls on one page, many times what any
    // other page draws (nobody opens it but the owner tuning the look). The nav's one row more, Components, raised
    // every other page by 91, and Feature flags by 464 more with the flag's switch row.
    ("Components", 55267),
    // The same row is one more for the search to draw; the four keys of the Changed files choices, which
    // every page reads as JSON each frame, are about 14 more.
    // The search finds the Borders row too, as one more result: 196 more.
    // The window miniature it shows now fills the stage: 9,923 measured.
    // The Image memory row is one more result for the search to draw: 565 more.
    // The same switch is one more entry for a search to list: 11,245 measured.
    ("search", 11225),
];

/// What the Changed page is measured with: five settings that differ from their defaults, on four pages
/// (Appearance, Terminal, Sessions, Git).
fn change(s: &mut crate::settings::Settings) {
    let defaults = crate::settings::Settings::default();
    s.blur = !defaults.blur;
    s.tab_width = 8;
    s.diff_wrap = !defaults.diff_wrap;
    s.minimap = !defaults.minimap;
    s.sidebar_changes = !defaults.sidebar_changes;
}

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
        // The Changed page lists what differs; every other page is measured with nothing different, as it is
        // for most people (a changed row draws a dot, a reset arrow and a closure more).
        let kept = cx.update(|cx| settings(cx).clone());
        if section == Section::Changed {
            cx.update(|cx| update_settings(cx, change));
        }
        cx.update(|cx| page.update(cx, |page, cx| page.show_section(section, cx)));
        cx.run_until_parked();
        if section == Section::Import {
            // The page's own scan has run over the scratch home and found nothing; measure the page
            // that has something to show: the cards and the open one's rows.
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
            cx.update(|cx| update_settings(cx, |s| *s = kept));
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

/// Allocations a third draw of the Motion preview makes on its own (a window holding nothing else), when the
/// budget was set: what a tick costs, since a scene builds only what shows at its moment of the loop. The
/// preview shows all four scenes at once (they cost 258, 434, 312 and 358 each, alone, as one stage), redrawn
/// by its own clock a few times a second; the scenes are drawn at Dense rows (Status at Regular): about
/// 1,500 for four, still hundreds each.
const PREVIEW_BUDGET: u64 = 1546;

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
    // The preview alone: its draw is what its clock asks for.
    let window = cx.add_window(|_, cx| MotionPreview::new(Scene::Insert, cx));
    let handle: AnyWindowHandle = window.into();
    draw(cx, handle);
    draw(cx, handle);
    let before = thread_counts();
    draw(cx, handle);
    let made = thread_counts().since(before).allocations;
    assert!(made <= PREVIEW_BUDGET + SLACK, "the preview drew {made} (budget {})", PREVIEW_BUDGET + SLACK);
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

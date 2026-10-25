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
//! Liquid Glass moved from Appearance (one row fewer) to Feature flags (one row more).
//! Appearance and a search (which show the window's preview) rose by about 770 when the preview
//! became the window's own parts at full size instead of a drawing of them.
//! Updates rose by about 380 with its one more row, Count this Mac anonymously, and by about 150 with
//! the note under it: what a check sends, and in a fresh data dir when counting starts.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.

use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{AnyWindowHandle, TestAppContext};
use wormhole_core::config::settings_index::Query;

use super::{Section, current_window, open_settings};
use crate::perf::allocations::thread_counts;
use crate::window::test_harness::{isolate_in, start};

/// Allocations a frame may make over what the pages made when this budget was set.
const SLACK: u64 = 20;

/// Allocations a third draw of each page made when the budget was set (the first two build what a window
/// keeps), and of the results page of a search for a word found in several pages; a frame may make
/// [`SLACK`] more.
const BUDGET: [(&str, u64); 14] = [
    ("Profile", 6590),
    ("Appearance", 15218),
    ("Fonts", 7288),
    ("Terminal", 9588),
    ("Sessions", 13516),
    ("Git", 4396),
    ("Agents", 26295),
    ("Notifications", 7711),
    ("MenuBar", 6024),
    ("Updates", 3779),
    ("Advanced", 3949),
    ("Projects", 8866),
    ("FeatureFlags", 3855),
    ("search", 12240),
];

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

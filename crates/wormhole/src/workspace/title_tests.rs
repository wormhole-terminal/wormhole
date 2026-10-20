//! View tests of the terminal's title row: what it keeps as the
//! window narrows, and the folder's menu.

use std::path::PathBuf;

use gpui_kit::{Modifiers, TestAppContext, VisualTestContext, point, px, size};
use wormhole_core::agents::AgentId;
use wormhole_core::layout::title_row::{Extra, PathShown};
use wormhole_core::usage::Gauge;

use super::view_tests::{draw, drive, isolate_in, start};
use crate::native::record;

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// A wide row shows the whole path, a narrower one drops parents out of its middle, and a narrow
/// one folds Open in… and the splits into one menu before the name is cut.
#[gpui_kit::test]
fn a_long_path_loses_its_middle_before_the_buttons_fold(cx: &mut TestAppContext) {
    if isolate("a_long_path_loses_its_middle_before_the_buttons_fold") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    let deep = PathBuf::from("/Volumes/Work/clients/acme/projects/backend/services/ingest-worker");
    let plan = |cx: &mut TestAppContext, width: f32| {
        let deep = deep.clone();
        drive(cx, &window, move |ws, window, cx| {
            ws.panes.panel_wanted = false;
            ws.window_width = width;
            ws.sessions[0].terminal().update(cx, |terminal, _| terminal.cwd = Some(deep));
            let plan = ws.title_plan(window, cx);
            let shown = match plan.fit.path {
                PathShown::Step(ix) => Some(plan.steps[ix].text.full()),
                PathShown::Icon | PathShown::None => None,
            };
            (shown, plan.fit.path, plan.fit.folded, plan.fit.name)
        })
    };

    let (shown, _, folded, _) = plan(cx, 3000.0);
    assert_eq!(shown.as_deref(), Some("/Volumes/Work/clients/acme/projects/backend/services/ingest-worker"));
    assert!(!folded);

    let (shown, path, folded, _) = plan(cx, 1100.0);
    let shown = shown.expect("the path is still drawn");
    assert!(matches!(path, PathShown::Step(ix) if ix > 0), "{shown}");
    assert!(shown.starts_with("/Volumes/…/") && shown.ends_with("/ingest-worker"), "{shown}");
    assert!(!folded, "the buttons stay while a path can give way: {shown}");

    // Narrower: the buttons fold into the one menu, and at the width where they first do the name
    // is still whole (it is cut only after that).
    let natural = plan(cx, 3000.0).3;
    let first = (500..=1100).rev().step_by(10).find(|width| plan(cx, *width as f32).2);
    let first = first.expect("a narrow enough row folds its buttons");
    let (_, path, folded, name) = plan(cx, first as f32);
    assert!(folded && name == natural, "at {first}: folded {folded}, name {name} of {natural}");
    assert!(matches!(path, PathShown::Step(ix) if ix > 0), "the path was cut before the buttons folded");
}

/// A click on the folder opens its menu (path, Open in Finder, Copy path, Insert into terminal and
/// the folders down to the volume); Copy path puts the path on the clipboard, and nothing about
/// the pointer resting on the folder opens anything.
#[gpui_kit::test]
fn clicking_the_folder_opens_its_menu_and_copy_path_copies_it(cx: &mut TestAppContext) {
    if isolate("clicking_the_folder_opens_its_menu_and_copy_path_copies_it") {
        return;
    }
    let window = start(cx, 1);
    cx.simulate_window_resize(window.handle, size(px(1200.0), px(800.0)));
    let folder = PathBuf::from("/Volumes/Work/clients/acme");
    let set = folder.clone();
    drive(cx, &window, move |ws, _, cx| {
        ws.sessions[0].terminal().update(cx, |terminal, _| terminal.cwd = Some(set));
    });
    draw(cx, &window);
    draw(cx, &window);
    record::close();

    let mut pointer = VisualTestContext::from_window(window.handle, cx);
    let at = pointer.debug_bounds("crumb-folder").expect("the folder is drawn").center();
    pointer.simulate_mouse_move(at, None, Modifiers::none());
    pointer.simulate_mouse_move(point(at.x + px(1.0), at.y), None, Modifiers::none());
    cx.run_until_parked();
    assert!(record::open().is_none(), "resting on the folder opens nothing");

    let mut pointer = VisualTestContext::from_window(window.handle, cx);
    pointer.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    let menu = record::open().expect("the click opened the menu");
    let outline = menu.outline();
    assert!(outline[0].starts_with("# "), "the path heads the menu: {outline:?}");
    assert_eq!(
        &outline[1..6],
        ["---", "- Open in Finder", "- Copy path", "- Insert into terminal", "---"],
        "{outline:?}"
    );
    let name = folder.file_name().expect("a folder with a name").to_string_lossy().into_owned();
    assert_eq!(outline[6], format!("- {name}"), "the folder itself comes first: {outline:?}");

    let (_, callbacks) = record::pick("Copy path").expect("the item is in the menu");
    cx.update(|cx| {
        window.handle.update(cx, |_, window, cx| {
            for run in callbacks {
                run(window, cx);
            }
        })
    })
    .expect("the window is open");
    cx.run_until_parked();
    let copied = cx.update(|cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert_eq!(copied.as_deref(), Some(folder.to_string_lossy().as_ref()));
    assert!(record::open().is_none(), "and the menu is closed");
}

/// A narrowing row gives up its extras after the path and before anything else, the ports a chip
/// at a time and then whole, and before the model and context do: at no width are the
/// ports there and the gauge gone, and while any extra is missing the path is cut as far as it
/// goes and the buttons have not folded.
#[gpui_kit::test]
fn the_extras_give_way_after_the_path_and_in_their_order(cx: &mut TestAppContext) {
    if isolate("the_extras_give_way_after_the_path_and_in_their_order") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    let gauge = Gauge { agent: AgentId::Claude, model: Some("Opus 4.5".into()), context: Some(41.0) };
    drive(cx, &window, move |ws, _, cx| {
        ws.panes.panel_wanted = false;
        ws.sessions[0].terminal().update(cx, |terminal, _| {
            terminal.cwd = Some(PathBuf::from("/Volumes/Work/clients/acme/projects/backend"));
            terminal.agent = Some(AgentId::Claude);
        });
        ws.handle_gauge("1", gauge, cx);
        ws.sessions[0].extras.found_ports(vec![3000, 5173, 8080, 9229]);
    });
    // [gauge, ports, chips of the ports, path at its last step, folded] at a width.
    let at = |cx: &mut TestAppContext, width: f32| {
        drive(cx, &window, move |ws, window, cx| {
            ws.window_width = width;
            let plan = ws.title_plan(window, cx);
            let last_step = match plan.fit.path {
                PathShown::Step(ix) => ix + 1 == plan.steps.len(),
                PathShown::Icon | PathShown::None => true,
            };
            let ports = plan.fit.extras.shows(Extra::Ports);
            let chips = if ports { 2usize.saturating_sub(plan.fit.ports_step).max(1) } else { 0 };
            (plan.fit.extras.shows(Extra::Gauge), ports, chips, last_step, plan.fit.folded)
        })
    };
    let rows: Vec<_> = (400..=2400).rev().step_by(25).map(|width| (width, at(cx, width as f32))).collect();
    let (_, whole) = rows[0];
    assert_eq!((whole.0, whole.1, whole.2), (true, true, 2), "everything fits a wide window");
    let mut chips_seen = Vec::new();
    for (width, (gauge, ports, chips, last_step, folded)) in &rows {
        assert!(!(*ports && !*gauge), "at {width}: the ports outlast the gauge");
        if !(*gauge && *ports && *chips == 2) {
            assert!(*last_step, "at {width}: an extra went before the path was cut as far as it goes");
        }
        if *gauge || *ports {
            assert!(!*folded, "at {width}: the buttons folded while extras were still drawn");
        }
        if *ports && !chips_seen.contains(chips) {
            chips_seen.push(*chips);
        }
    }
    assert_eq!(chips_seen, [2, 1], "the ports go from two chips to one before they are gone");
    let (_, narrow) = *rows.last().expect("widths");
    assert!(!narrow.0 && !narrow.1, "a narrow window has none of them");
}

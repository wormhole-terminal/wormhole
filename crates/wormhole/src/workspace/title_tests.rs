//! View tests of the terminal's title row: what it keeps as the
//! window narrows, and the path's hover card.

use std::path::PathBuf;
use std::time::Duration;

use gpui_kit::TestAppContext;
use wormhole_core::agents::AgentId;
use wormhole_core::layout::title_row::{Extra, PathShown};
use wormhole_core::usage::Gauge;

use super::path_card::Over;
use super::view_tests::{draw, drive, isolate_in, read, start};

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

    let (shown, path, folded, _) = plan(cx, 1000.0);
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

/// Resting on the path opens the card after a moment; it stays while the pointer is on the path or
/// the card, closes a moment after both are left, and Esc closes it at once.
#[gpui_kit::test]
fn resting_on_the_path_opens_its_card(cx: &mut TestAppContext) {
    if isolate("resting_on_the_path_opens_its_card") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    let shown = |cx: &mut TestAppContext| read(cx, &window, |ws| ws.path_card.is_shown());
    let wait = |cx: &mut TestAppContext, ms: u64| {
        cx.executor().advance_clock(Duration::from_millis(ms));
        cx.run_until_parked();
    };

    drive(cx, &window, |ws, _, cx| ws.hover_path(Over::Path, true, cx));
    wait(cx, 300);
    assert!(!shown(cx), "not yet: crossing the path on the way elsewhere opens nothing");
    wait(cx, 200);
    assert!(shown(cx), "after about 400 ms on the path");

    // Moving from the path onto the card: it stays through the gap between them.
    drive(cx, &window, |ws, _, cx| ws.hover_path(Over::Path, false, cx));
    wait(cx, 100);
    drive(cx, &window, |ws, _, cx| ws.hover_path(Over::Card, true, cx));
    wait(cx, 1000);
    assert!(shown(cx), "while the pointer is on the card");

    drive(cx, &window, |ws, _, cx| ws.hover_path(Over::Card, false, cx));
    wait(cx, 100);
    assert!(shown(cx), "a moment of grace");
    wait(cx, 200);
    assert!(!shown(cx), "closed once both are left");

    // Esc closes it at once, and only when it is open.
    drive(cx, &window, |ws, _, cx| ws.hover_path(Over::Path, true, cx));
    wait(cx, 500);
    assert!(shown(cx));
    assert!(drive(cx, &window, |ws, _, cx| ws.close_path_card(cx)));
    assert!(!shown(cx));
    assert!(!drive(cx, &window, |ws, _, cx| ws.close_path_card(cx)));
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

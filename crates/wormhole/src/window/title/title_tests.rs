//! View tests of the terminal's title row: what it keeps as the
//! window narrows, and the folder's menu.

use std::path::PathBuf;

use std::time::Duration;

use gpui_kit::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px, size};
use wormhole_core::agents::AgentId;
use wormhole_core::agents::usage::Gauge;
use wormhole_core::view::layout::title_row::{Extra, PathShown};

use crate::platform::record;
use crate::window::view_tests::{draw, drive, isolate_in, start};

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

/// Resting on the folder opens its popover after a moment (the path and three buttons); it stays
/// while the pointer is on the folder or on the popover and closes a moment after both are left.
/// Copy path copies and says "Copied"; a right-click on the folder opens the Finder-style path
/// menu instead.
#[gpui_kit::test]
fn resting_on_the_folder_opens_its_popover_and_a_right_click_its_menu(cx: &mut TestAppContext) {
    if isolate("resting_on_the_folder_opens_its_popover_and_a_right_click_its_menu") {
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
    record::popover_close(false);
    let wait = |cx: &mut TestAppContext, ms: u64| {
        cx.executor().advance_clock(Duration::from_millis(ms));
        cx.run_until_parked();
    };

    let mut pointer = VisualTestContext::from_window(window.handle, cx);
    let at = pointer.debug_bounds("crumb-folder").expect("the folder is drawn").center();
    pointer.simulate_mouse_move(at, None, Modifiers::none());
    pointer.simulate_mouse_move(point(at.x + px(1.0), at.y), None, Modifiers::none());
    wait(cx, 300);
    assert!(record::open_popover().is_none(), "not yet: crossing the folder opens nothing");
    wait(cx, 200);
    let popover = record::open_popover().expect("after about 400 ms on the folder");
    assert_eq!(popover.outline()[0], "popover hover");
    assert!(popover.outline()[1].starts_with("# "));
    assert_eq!(
        popover.outline()[2],
        "buttons: [Open in Finder] [Copy path -> Copied] [Insert into terminal]"
    );

    // Copy path copies the plain path, and the button says so; the popover stays.
    let (_, callbacks) = record::popover_press("Copy path").expect("the button");
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
    // (The terminal reads its own folder from the process as time passes: whichever it has.)
    let now = drive(cx, &window, |ws, _, cx| ws.sessions[0].terminal().read(cx).cwd.clone());
    assert_eq!(copied.as_deref(), now.as_deref().and_then(|path| path.to_str()));
    assert!(record::open_popover().expect("still open").outline()[2].contains("[Copied "), "it says Copied");

    // Off the folder and not onto the popover: it goes a moment later. Onto the popover: it stays.
    let mut pointer = VisualTestContext::from_window(window.handle, cx);
    pointer.simulate_mouse_move(point(px(900.0), px(600.0)), None, Modifiers::none());
    wait(cx, 100);
    assert!(record::open_popover().is_some(), "a moment of grace");
    let on_hover =
        record::open_popover().and_then(|popover| popover.on_hover).expect("it reports the pointer");
    cx.update(|cx| window.handle.update(cx, |_, window, cx| on_hover(true, window, cx))).expect("open");
    wait(cx, 1000);
    assert!(record::open_popover().is_some(), "while the pointer is on the popover");
    cx.update(|cx| window.handle.update(cx, |_, window, cx| on_hover(false, window, cx))).expect("open");
    wait(cx, 300);
    assert!(record::open_popover().is_none(), "closed once both are left");

    // The right-click is the path menu.
    let set = folder.clone();
    drive(cx, &window, move |ws, _, cx| {
        ws.sessions[0].terminal().update(cx, |terminal, _| terminal.cwd = Some(set));
    });
    draw(cx, &window);
    draw(cx, &window);
    let mut pointer = VisualTestContext::from_window(window.handle, cx);
    pointer.simulate_mouse_move(at, None, Modifiers::none());
    pointer.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
    pointer.simulate_mouse_up(at, MouseButton::Right, Modifiers::none());
    cx.run_until_parked();
    let menu = record::open().expect("the right-click opened the menu");
    let outline = menu.outline();
    assert!(outline[0].starts_with("# "), "the path heads the menu: {outline:?}");
    assert_eq!(
        &outline[1..6],
        ["---", "- Open in Finder", "- Copy path", "- Insert into terminal", "---"],
        "{outline:?}"
    );
    assert_eq!(outline[6], "- acme", "the folder itself comes first: {outline:?}");
    assert_eq!(&outline[7..], ["- clients", "- Work"], "down to the volume it is on");
    assert!(record::open_popover().is_none(), "and no popover with it");
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

// ---------------------------------------------------------------- the pull request chip

/// The chip shows a pull request only for a branch it may ask `gh` about: the switch is on, the
/// terminal's folder is in a repository on a branch git has answered for, and its project is trusted
/// (`gh` reads the repository's own configuration, which a repository the person has not approved may
/// have filled with anything). What it shows is what the app has cached for the branch, and its words
/// are VoiceOver's.
#[gpui_kit::test]
fn the_pull_request_chip_shows_a_cached_pull_request_only_for_an_approved_branch(cx: &mut TestAppContext) {
    use std::time::Instant;

    use wormhole_core::git::RepoStatus;
    use wormhole_core::git::pr::{Branch, Checks, PrState, PullRequest};
    if isolate("the_pull_request_chip_shows_a_cached_pull_request_only_for_an_approved_branch") {
        return;
    }
    let window = start(cx, 1);
    let root = std::env::current_dir().expect("the scratch dir").join("pr-repo");
    std::fs::create_dir_all(root.join(".wormhole")).expect("mkdir");
    let root = root.canonicalize().expect("canonical");
    std::fs::write(root.join(".wormhole").join("actions.json"), "[]").expect("write");
    let status = RepoStatus {
        root: root.clone(),
        branch: Some("feature".into()),
        files: Vec::new(),
        base_rev: "HEAD".into(),
        compare: None,
        compare_missing: None,
        dirty: false,
        upstream: None,
        superproject: None,
    };
    drive(cx, &window, {
        let (root, status) = (root.clone(), status.clone());
        move |ws, _, cx| {
            ws.sessions[0].terminal().update(cx, |terminal, _| terminal.cwd = Some(root.clone()));
            ws.sessions[0].git.status = Some(status);
            ws.sessions[0].git.cwd = Some(root);
            ws.refresh_features(cx);
        }
    });
    let branch = Branch { root: root.clone(), name: "feature".into() };
    let pr = PullRequest {
        number: 142,
        url: "https://example.com/pull/142".into(),
        state: PrState::Open,
        draft: false,
        checks: Some(Checks::Failing),
        review: None,
    };
    cx.update(|cx| {
        let cache = &mut crate::app_state::app_mut(cx).pr;
        assert!(cache.begin(&branch, Instant::now()));
        cache.finish(&branch, Ok(Some(pr.clone())), Instant::now());
    });
    let shown = |cx: &mut TestAppContext| {
        crate::window::view_tests::read_cx(cx, &window, |ws, cx| {
            ws.extra_facts(cx).pull_request.map(|pr| (pr.label(), pr.words()))
        })
    };
    assert_eq!(shown(cx), None, "the project is not trusted: no lookup, and nothing shown");
    cx.update(|cx| {
        let folder = wormhole_core::project::ProjectFolder::open(&root).expect("a project");
        crate::features::project::hub_mut(cx).trust.trust(&folder);
    });
    drive(cx, &window, |ws, _, cx| ws.refresh_features(cx));
    let (label, words) = shown(cx).expect("trusted: the cached pull request");
    assert_eq!(label, "#142");
    assert_eq!(words, pr.words());
    assert!(words.starts_with("Pull request #142, open"), "{words}");

    cx.update(|cx| crate::settings::update_settings(cx, |settings| settings.title_pull_request = false));
    assert_eq!(shown(cx), None, "the switch is off");
    cx.update(|cx| crate::settings::update_settings(cx, |settings| settings.title_pull_request = true));
    // A branch git has not answered for (another folder's answer) shows nothing.
    drive(cx, &window, |ws, _, _| ws.sessions[0].git.cwd = Some(PathBuf::from("/elsewhere")));
    assert_eq!(shown(cx), None);
}

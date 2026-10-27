//! The intro window in a real window (GPUI's test platform; `test_harness` says how each test runs in
//! a child process with scratch data): its pages walked with the keys, Esc, ⇥ staying among its
//! buttons, a replay coming back to the welcome page, and the intro opened again later.

use std::time::Duration;

use std::path::PathBuf;

use gpui_kit::TestAppContext;
use wormhole_core::agents::AgentId;
use wormhole_core::agents::integrations;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::config::settings_index::SectionId;
use wormhole_core::onboarding::{Event, ONBOARDED_KEY, Step};
use wormhole_core::project::{ProjectFolder, TrustStatus};
use wormhole_core::storage::store::Store;

use crate::app_state::{OpenWindow, app, app_mut};
use crate::onboarding::{AgentsNote, FADE_OUT, FirstRun};
use crate::settings::update_settings;
use crate::window::test_harness::{
    draw, draw_fresh, drive, isolate_in, press, read, read_cx, start, with_window,
};

/// Press a key as the keyboard would, after a frame (the keys go where the last frame's focus is).
fn key(cx: &mut TestAppContext, window: &OpenWindow, keys: &str) {
    draw(cx, window);
    press(cx, window, keys);
}

/// The window's first run, to read.
fn first_run<R>(cx: &mut TestAppContext, window: &OpenWindow, f: impl FnOnce(&FirstRun) -> R) -> R {
    read_cx(cx, window, |ws, cx| f(ws.first_run.read(cx)))
}

/// The window's first run, to change, as a window's update would.
fn first_run_mut(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    f: impl FnOnce(&mut FirstRun, &mut gpui_kit::Context<FirstRun>),
) {
    drive(cx, window, |ws, _, cx| ws.first_run.update(cx, f));
}

fn step(cx: &mut TestAppContext, window: &OpenWindow) -> Option<Step> {
    first_run(cx, window, |first_run| first_run.intro.as_ref().map(|intro| intro.model.step()))
}

/// Whether the keyboard is on the page or one of its buttons.
fn in_page(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
    draw(cx, window);
    cx.update(|cx| {
        window.handle.update(cx, |_, win, cx| {
            let workspace = window.workspace.upgrade()?;
            let first_run = workspace.read(cx).first_run.clone();
            Some(first_run.read(cx).intro.as_ref()?.focus.contains_focused(win, cx))
        })
    })
    .ok()
    .flatten()
    .unwrap_or(false)
}

#[gpui_kit::test]
fn the_arrows_and_return_walk_the_pages_and_esc_leaves(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_arrows_and_return_walk_the_pages_and_esc_leaves") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.open_onboarding(Step::Welcome, window, cx));
    assert_eq!(step(cx, &window), Some(Step::Welcome));
    assert!(first_run(cx, &window, FirstRun::is_open));

    key(cx, &window, "right");
    assert_eq!(step(cx, &window), Some(Step::Folder), "→: the next page");
    key(cx, &window, "left");
    assert_eq!(step(cx, &window), Some(Step::Welcome), "←: back");
    key(cx, &window, "left");
    assert_eq!(step(cx, &window), Some(Step::Welcome), "← on the first page stays");
    key(cx, &window, "enter");
    assert_eq!(step(cx, &window), Some(Step::Folder), "⏎: the main button, Get started");

    // The window's own shortcuts wait while the intro is up: ⌘T opens no terminal under it.
    key(cx, &window, "cmd-t");
    read(cx, &window, |ws| assert_eq!(ws.sessions.len(), 1, "⌘T waits"));

    key(cx, &window, "escape");
    assert_eq!(step(cx, &window), None, "Esc skips the intro");
    assert!(!first_run(cx, &window, FirstRun::is_open));
}

#[gpui_kit::test]
fn tab_stays_among_the_pages_buttons(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "tab_stays_among_the_pages_buttons") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.open_onboarding(Step::Welcome, window, cx));
    // Past the last of its buttons, in both directions, and the keys are still on the page: the
    // sidebar under it is out of reach.
    for _ in 0..8 {
        key(cx, &window, "tab");
        assert!(in_page(cx, &window), "⇥ left the page");
    }
    for _ in 0..8 {
        key(cx, &window, "shift-tab");
        assert!(in_page(cx, &window), "⇧⇥ left the page");
    }
}

#[gpui_kit::test]
fn a_replay_comes_back_to_the_welcome_page(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_replay_comes_back_to_the_welcome_page") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.open_onboarding(Step::Look, window, cx));
    assert_eq!(step(cx, &window), Some(Step::Look));

    key(cx, &window, "r");
    // The window leaves first; the page changes once it is out of sight.
    assert_eq!(step(cx, &window), Some(Step::Look), "still the old page while the window leaves");
    cx.executor().advance_clock(FADE_OUT + Duration::from_millis(50));
    cx.run_until_parked();
    assert_eq!(step(cx, &window), Some(Step::Welcome), "R: back to the welcome page");
    assert!(in_page(cx, &window), "with the keyboard on it");
}

#[gpui_kit::test]
fn the_intro_can_be_shown_again_as_the_welcome_page_alone(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_intro_can_be_shown_again_as_the_welcome_page_alone") {
        return;
    }
    let window = start(cx, 1);
    // With the terminal welcome off (it is on by default in a development build), Help → Welcome
    // to Wormhole is the intro window, as ever.
    cx.update(|cx| {
        update_settings(cx, |s| {
            s.feature_flags.insert(FeatureFlag::TerminalWelcome.key().to_string(), false);
        })
    });
    drive(cx, &window, |ws, window, cx| ws.show_intro(window, cx));
    // A window that is showing leaves first, as for a replay.
    cx.executor().advance_clock(FADE_OUT + Duration::from_millis(50));
    cx.run_until_parked();
    assert_eq!(step(cx, &window), Some(Step::Welcome));
    first_run(cx, &window, |first_run| {
        let intro = first_run.intro.as_ref().expect("open");
        assert!(!intro.first_run, "a replay, not the first run");
        assert!(intro.model.is_last(), "its one button ends it");
    });
    // → does not end it, ⏎ does; and afterwards the terminal has the keyboard.
    key(cx, &window, "right");
    assert_eq!(step(cx, &window), Some(Step::Welcome));
    key(cx, &window, "enter");
    assert_eq!(step(cx, &window), None);
    first_run(cx, &window, |first_run| assert!(first_run.agents_note == crate::onboarding::AgentsNote::Off));
}

// ------------------------------------------------------------------------------ the first run

/// The scratch database, opened as the app does at launch.
fn open_store(cx: &mut TestAppContext) {
    cx.update(|cx| app_mut(cx).store = Some(Store::open().expect("opens the scratch database")));
}

/// Whether the first run was remembered as dealt with.
fn remembered(cx: &mut TestAppContext) -> bool {
    cx.update(|cx| {
        let store = app(cx).store.as_ref().expect("a store");
        store.meta(ONBOARDED_KEY).ok().flatten().as_deref() == Some("1")
    })
}

fn terminal_flag(cx: &mut TestAppContext, on: bool) {
    cx.update(|cx| {
        update_settings(cx, |s| {
            s.feature_flags.insert(FeatureFlag::TerminalWelcome.key().to_string(), on);
        })
    });
}

/// A folder in the test's scratch dir.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::current_dir().expect("the scratch dir").join(name);
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir
}

/// The agents the third step lists are looked for on the real PATH, which a test must not depend on:
/// say whether any was found.
fn agents_on_path(cx: &mut TestAppContext, window: &OpenWindow, found: bool) {
    first_run_mut(cx, window, |first_run, _| {
        if let Some(intro) = &mut first_run.intro {
            intro.model.set_agents_found(found);
        }
    });
}

/// Press the buttons: an event, as a click or a key makes it.
fn event(cx: &mut TestAppContext, window: &OpenWindow, event: Event) {
    drive(cx, window, |ws, window, cx| {
        ws.first_run.update(cx, |first_run, cx| first_run.onboarding_event(event, window, cx))
    });
}

#[gpui_kit::test]
fn the_first_launch_opens_the_intro_window_when_the_terminal_welcome_is_off(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_first_launch_opens_the_intro_window_when_the_terminal_welcome_is_off")
    {
        return;
    }
    let window = start(cx, 1);
    open_store(cx);
    terminal_flag(cx, false);
    assert!(cx.update(|cx| crate::app::welcome::first_launch_plan(cx)).is_none(), "no terminal welcome");

    cx.update(crate::app::welcome::start_if_first_launch);
    cx.run_until_parked();
    assert_eq!(step(cx, &window), Some(Step::Welcome), "the intro window, on its welcome page");
    first_run(cx, &window, |first_run| {
        let intro = first_run.intro.as_ref().expect("open");
        assert!(intro.first_run, "the first run, not a replay");
        assert!(!intro.model.is_last(), "with its steps to come");
    });
    assert!(!remembered(cx), "nothing is remembered before it is dealt with");
}

#[gpui_kit::test]
fn the_first_launch_is_the_terminal_welcome_while_its_flag_is_on(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_first_launch_is_the_terminal_welcome_while_its_flag_is_on") {
        return;
    }
    let window = start(cx, 1);
    open_store(cx);
    terminal_flag(cx, true);
    let plan = cx.update(|cx| crate::app::welcome::first_launch_plan(cx)).expect("a plan");
    assert!(plan.first_run);
    // The plan is the window's own first terminal (the launch gives it to the window it opens); the
    // intro window does not open for it.
    drive(cx, &window, |_, _, _| {});
    assert_eq!(step(cx, &window), None);
}

#[gpui_kit::test]
fn a_first_run_that_was_dealt_with_is_not_shown_again(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_first_run_that_was_dealt_with_is_not_shown_again") {
        return;
    }
    let window = start(cx, 1);
    open_store(cx);
    cx.update(|cx| app(cx).store.as_ref().expect("a store").set_meta(ONBOARDED_KEY, "1").expect("saves"));
    for on in [false, true] {
        terminal_flag(cx, on);
        assert!(cx.update(|cx| crate::app::welcome::first_launch_plan(cx)).is_none());
    }
    terminal_flag(cx, false);
    cx.update(crate::app::welcome::start_if_first_launch);
    cx.run_until_parked();
    assert_eq!(step(cx, &window), None, "no intro window");
}

#[gpui_kit::test]
fn the_steps_walk_to_start_which_remembers_the_run_and_moves_the_first_terminal(cx: &mut TestAppContext) {
    if isolate_in(
        module_path!(),
        "the_steps_walk_to_start_which_remembers_the_run_and_moves_the_first_terminal",
    ) {
        return;
    }
    let window = start(cx, 1);
    open_store(cx);
    let work = folder("first-run-folder");
    drive(cx, &window, |ws, window, cx| ws.open_onboarding(Step::Welcome, window, cx));
    agents_on_path(cx, &window, true);

    event(cx, &window, Event::Continue);
    assert_eq!(step(cx, &window), Some(Step::Folder));
    event(cx, &window, Event::PickFolder(work.clone()));
    assert_eq!(step(cx, &window), Some(Step::Folder), "picking a folder stays on the step");
    event(cx, &window, Event::Continue);
    assert_eq!(step(cx, &window), Some(Step::Look));
    event(cx, &window, Event::Continue);
    assert_eq!(step(cx, &window), Some(Step::Agents));
    event(cx, &window, Event::Back);
    assert_eq!(step(cx, &window), Some(Step::Look), "Back goes back a page");
    event(cx, &window, Event::Continue);
    assert!(!remembered(cx), "nothing is remembered until the end");
    read(cx, &window, |ws| assert_eq!(ws.sessions[0].id, 1));

    event(cx, &window, Event::Start);
    assert_eq!(step(cx, &window), None, "Start closes the intro window");
    assert!(remembered(cx), "and remembers the first run");
    cx.update(|cx| assert!(app(cx).store.is_some()));
    read_cx(cx, &window, |ws, cx| {
        assert_eq!(ws.sessions.len(), 1);
        assert_ne!(ws.sessions[0].id, 1, "the first terminal was replaced by one in the folder");
        assert_eq!(ws.sessions[0].terminal().read(cx).cwd.as_deref(), Some(work.as_path()));
    });
    first_run(cx, &window, |first_run| {
        assert!(first_run.agents_note == AgentsNote::Off, "agents were found: no note")
    });
    // Start itself installs nothing: the agents' home is as empty as before.
    let home = integrations::home();
    assert!(
        !home.exists() || std::fs::read_dir(&home).map_or(0, |d| d.count()) == 0,
        "Start installed hooks"
    );
}

#[gpui_kit::test]
fn skipping_the_first_run_remembers_it_and_leaves_the_terminal_where_it_is(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "skipping_the_first_run_remembers_it_and_leaves_the_terminal_where_it_is") {
        return;
    }
    let window = start(cx, 1);
    open_store(cx);
    let work = folder("skipped-folder");
    drive(cx, &window, |ws, window, cx| ws.open_onboarding(Step::Folder, window, cx));
    event(cx, &window, Event::PickFolder(work));
    event(cx, &window, Event::Skip);
    assert_eq!(step(cx, &window), None);
    assert!(remembered(cx), "skipped is dealt with too");
    read(cx, &window, |ws| assert_eq!(ws.sessions[0].id, 1, "the folder picked is not started in"));
}

#[gpui_kit::test]
fn a_replay_of_the_intro_remembers_nothing(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_replay_of_the_intro_remembers_nothing") {
        return;
    }
    let window = start(cx, 1);
    open_store(cx);
    terminal_flag(cx, false);
    drive(cx, &window, |ws, window, cx| ws.show_intro(window, cx));
    cx.executor().advance_clock(FADE_OUT + Duration::from_millis(50));
    cx.run_until_parked();
    event(cx, &window, Event::Start);
    assert_eq!(step(cx, &window), None);
    assert!(!remembered(cx), "only the first run is remembered");
    read(cx, &window, |ws| assert_eq!(ws.sessions[0].id, 1, "and no terminal moves"));
}

#[gpui_kit::test]
fn ending_on_the_primer_puts_up_the_no_agents_note(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "ending_on_the_primer_puts_up_the_no_agents_note") {
        return;
    }
    let window = start(cx, 1);
    open_store(cx);
    drive(cx, &window, |ws, window, cx| ws.open_onboarding(Step::Primer, window, cx));
    assert_eq!(step(cx, &window), Some(Step::Primer));
    event(cx, &window, Event::Start);
    assert_eq!(step(cx, &window), None);
    assert!(remembered(cx));
    first_run(cx, &window, |first_run| assert!(first_run.agents_note == AgentsNote::Shown));
}

#[gpui_kit::test]
fn only_an_agents_own_install_writes_its_hooks(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "only_an_agents_own_install_writes_its_hooks") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.open_onboarding(Step::Agents, window, cx));
    agents_on_path(cx, &window, true);
    let home = integrations::home();
    let spec = integrations::spec(AgentId::Claude).expect("Claude has hooks");
    assert!(spec.state(&home).is_ok_and(|state| state != integrations::State::Installed));
    first_run_mut(cx, &window, |first_run, _| {
        // The list of agents a PATH lookup found is the test's own.
        if let Some(intro) = &mut first_run.intro {
            intro.agents = Some(crate::onboarding::Agents {
                found: vec![crate::onboarding::FoundAgent {
                    agent: wormhole_core::agents::AGENTS
                        .iter()
                        .find(|a| a.id == AgentId::Claude)
                        .expect("Claude is an agent"),
                    spec: Some(spec),
                    state: spec.state(&integrations::home()).ok(),
                    path: None,
                }],
                others: Vec::new(),
            });
        }
    });
    // Moving about the pages writes nothing.
    event(cx, &window, Event::Back);
    event(cx, &window, Event::Continue);
    assert!(spec.state(&home).is_ok_and(|state| state != integrations::State::Installed));

    event(cx, &window, Event::Install(AgentId::Claude));
    assert_eq!(spec.state(&home), Ok(integrations::State::Installed), "its Install button wrote them");
    first_run(cx, &window, |first_run| {
        let intro = first_run.intro.as_ref().expect("open");
        assert!(intro.problem.is_none());
        let found = &intro.agents.as_ref().expect("looked for").found;
        assert_eq!(found[0].state, Some(integrations::State::Installed), "and the row reads them back");
    });
    let other = integrations::spec(AgentId::Codex);
    if let Some(other) = other {
        assert!(
            other.state(&home).is_ok_and(|state| state != integrations::State::Installed),
            "only that one"
        );
    }
}

#[gpui_kit::test]
fn the_trust_button_asks_and_a_cancelled_ask_trusts_nothing(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_trust_button_asks_and_a_cancelled_ask_trusts_nothing") {
        return;
    }
    let window = start(cx, 1);
    let dir = folder("trust-folder");
    std::fs::create_dir_all(dir.join(".wormhole")).expect("mkdir");
    std::fs::write(
        dir.join(".wormhole/actions.json"),
        r#"[{"id": "t", "title": "T", "type": "run", "command": "make"}]"#,
    )
    .expect("write");
    drive(cx, &window, |ws, window, cx| ws.open_onboarding(Step::Agents, window, cx));
    // The intro looks for agents on the real PATH off the UI thread and moves the third step to the primer
    // when it finds none, which is what a CI runner finds: let that land, then say an agent is there, or
    // the button is not on the page and the press is ignored.
    cx.run_until_parked();
    agents_on_path(cx, &window, true);
    assert_eq!(step(cx, &window), Some(Step::Agents), "the trust card is on the agents step");
    event(cx, &window, Event::Trust(dir.clone()));
    cx.run_until_parked();
    let status =
        cx.update(|cx| crate::features::project::hub(cx).projects.get(&dir).map(|project| project.status));
    assert_eq!(status, Some(TrustStatus::Untrusted), "the unit-test build cancels the dialog");
    assert!(!cx.update(|cx| crate::features::project::hub(cx).approving), "and nothing waits");
    assert_eq!(step(cx, &window), Some(Step::Agents), "the intro stays");
    assert!(ProjectFolder::open(&dir).is_some());
}

#[cfg(feature = "devctl")]
#[gpui_kit::test]
fn devctls_close_leaves_the_intro_without_remembering_it(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "devctls_close_leaves_the_intro_without_remembering_it") {
        return;
    }
    let window = start(cx, 1);
    open_store(cx);
    drive(cx, &window, |ws, window, cx| ws.open_onboarding(Step::Look, window, cx));
    assert_eq!(step(cx, &window), Some(Step::Look));
    drive(cx, &window, |ws, window, cx| ws.close_onboarding(window, cx));
    assert_eq!(step(cx, &window), None);
    assert!(!remembered(cx), "closed, not finished");
}

#[gpui_kit::test]
fn the_notes_settings_link_is_an_action_the_app_answers_by_opening_settings(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_notes_settings_link_is_an_action_the_app_answers_by_opening_settings")
    {
        return;
    }
    let window = start(cx, 1);
    assert!(cx.update(|cx| crate::app_state::live(&app(cx).settings_window).is_none()));
    // What the link in the sidebar's "No coding agents yet" note dispatches from the window.
    with_window(cx, &window, |window, cx| {
        window.dispatch_action(Box::new(crate::actions::OpenSettingsAt(SectionId::Agents)), cx)
    });
    cx.run_until_parked();
    assert!(cx.update(|cx| crate::app_state::live(&app(cx).settings_window).is_some()), "Settings opened");
}

// -------------------------------------------------------------------------------- the frame

/// Allocations the third draw of a one-terminal window under the intro window's welcome page, and under
/// its look step (the page with the most to draw), may make. Measured: 3,417 and 5,127 allocations
/// (1.9 MB and 2.8 MB) while the intro was part of the `Workspace`, 3,419 and 5,129 as a child view of
/// its own. The look step's walk through how much to show added about 8,300 to it (13,458 and 9.6 MB):
/// three answers to a question, each a sketch of the window, and the words round them. The sketches were
/// first boxes, some forty to each and ~29,600 allocations in all; as one canvas each they are about 400
/// altogether, and a card of fixed height (not one that measures its text for its height) took most of
/// the rest. The budget is that plus about a hundredth, so a cost per card or per frame that grows shows
/// at once.
const WELCOME_FRAME_ALLOCATIONS: u64 = 3_455;
const LOOK_FRAME_ALLOCATIONS: u64 = 13_600;

#[gpui_kit::test]
fn a_frame_under_the_intro_window_stays_within_its_allocation_budget(cx: &mut TestAppContext) {
    use crate::perf::allocations::thread_counts;
    if isolate_in(module_path!(), "a_frame_under_the_intro_window_stays_within_its_allocation_budget") {
        return;
    }
    let window = start(cx, 1);
    for (page, budget) in [(Step::Welcome, WELCOME_FRAME_ALLOCATIONS), (Step::Look, LOOK_FRAME_ALLOCATIONS)] {
        drive(cx, &window, |ws, window, cx| ws.open_onboarding(page, window, cx));
        // The first draws build what a window keeps; the third is a frame.
        draw(cx, &window);
        draw(cx, &window);
        let before = thread_counts();
        draw_fresh(cx, &window);
        let drawn = thread_counts().since(before);
        eprintln!(
            "window frame under the intro's {page:?} page: {} allocations, {} bytes",
            drawn.allocations, drawn.bytes
        );
        assert!(
            drawn.allocations <= budget,
            "a frame under the {page:?} page made {} allocations (budget {budget})",
            drawn.allocations
        );
    }
}

// ------------------------------------------------------------------- how much to show (Look step)

/// What the window shows now: the settings' parts and whether the right panel is wanted.
fn shown_now(cx: &mut TestAppContext, window: &OpenWindow) -> wormhole_core::config::presets::Shown {
    let panel = read_cx(cx, window, |ws, _| ws.panes.panel_wanted);
    cx.update(|cx| {
        wormhole_core::config::presets::Shown::read(
            &cx.global::<crate::settings::SettingsStore>().settings,
            panel,
        )
    })
}

/// Where the walk is: the number of the question asked, or `None` on the summary or before it began.
fn question_number(cx: &mut TestAppContext, window: &OpenWindow) -> Option<usize> {
    first_run(cx, window, |first_run| {
        let intro = first_run.intro.as_ref()?;
        intro.tailor_walk()?.step().map(|step| step.number)
    })
}

#[gpui_kit::test]
fn the_digits_answer_the_look_steps_question_and_the_window_follows(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_digits_answer_the_look_steps_question_and_the_window_follows") {
        return;
    }
    use wormhole_core::config::presets::{Preset, Shown};
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.open_onboarding(Step::Look, window, cx));
    // The test window starts with its panel open; the rest is a first launch's.
    let first = shown_now(cx, &window);
    assert_eq!(Shown { panel_open: false, ..first.clone() }, Shown::of(Preset::Balanced));

    // Everything: the whole window, the panel opened by the window itself.
    key(cx, &window, "3");
    assert_eq!(shown_now(cx, &window), Shown::of(Preset::Everything));
    assert_eq!(question_number(cx, &window), Some(2), "on to the title row");

    // Zen, from the start again: everything away and the panel closed.
    drive(cx, &window, |ws, _, cx| ws.tailor_act(crate::onboarding::TailorAct::StartOver, cx));
    assert_eq!(shown_now(cx, &window), first, "start over puts back what was there");
    key(cx, &window, "1");
    assert_eq!(shown_now(cx, &window), Shown::of(Preset::Zen));
}

#[gpui_kit::test]
fn a_digit_past_the_answers_or_off_the_look_step_does_nothing(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_digit_past_the_answers_or_off_the_look_step_does_nothing") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.open_onboarding(Step::Folder, window, cx));
    key(cx, &window, "1");
    assert_eq!(question_number(cx, &window), None, "no walk on the folder step");

    drive(cx, &window, |ws, window, cx| ws.open_onboarding(Step::Look, window, cx));
    key(cx, &window, "1");
    key(cx, &window, "1");
    assert_eq!(question_number(cx, &window), Some(3), "Zen, then the title row's first answer");
    // The status bar has two answers: 3 is none.
    key(cx, &window, "3");
    assert_eq!(question_number(cx, &window), Some(3), "no third answer");
    // The sheet's own keys are as ever.
    key(cx, &window, "right");
    // (The tests see no installed agent, so the step after Look is the short primer.)
    assert_eq!(step(cx, &window), Some(Step::Primer), "→ is the sheet's Continue");
}

#[gpui_kit::test]
fn the_summary_is_accepted_with_looks_good_and_start_over_asks_again(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_summary_is_accepted_with_looks_good_and_start_over_asks_again") {
        return;
    }
    use crate::onboarding::TailorAct;
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.open_onboarding(Step::Look, window, cx));
    for _ in 0..7 {
        key(cx, &window, "1");
    }
    assert_eq!(question_number(cx, &window), None, "all seven answered: the summary");
    first_run(cx, &window, |first_run| {
        let walk = first_run.intro.as_ref().and_then(|intro| intro.tailor_walk()).expect("a walk");
        assert_eq!(walk.at, wormhole_core::onboarding::tailor::At::Done);
    });
    assert!(drive_act(cx, &window, TailorAct::Settle));
    assert!(first_run(cx, &window, |first_run| first_run.tailor_settled()));
    assert!(drive_act(cx, &window, TailorAct::StartOver));
    assert!(!first_run(cx, &window, |first_run| first_run.tailor_settled()));
    assert_eq!(question_number(cx, &window), Some(1));
}

fn drive_act(cx: &mut TestAppContext, window: &OpenWindow, act: crate::onboarding::TailorAct) -> bool {
    drive(cx, window, |ws, _, cx| ws.tailor_act(act, cx))
}

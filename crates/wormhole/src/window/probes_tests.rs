//! The Probes through a real window and the real event bus: an approved Probe comes into force, a bell on
//! the bus reaches its hook, turning it off or the flag off takes everything back and ends the listening.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`). Approving goes through the
//! same core calls as the sheet, with the development key file in that scratch folder (never the Keychain).

use std::fs;
use std::time::{Duration, Instant};

use gpui_kit::TestAppContext;
use wormhole_core::probe::approval::{self, Proof};
use wormhole_core::storage::paths::DataRoot;
use wormhole_term::PtyEvent;

use crate::probe_runtime;
use crate::settings::update_settings;
use crate::window::reported_tests::{event, terminal};
use crate::window::test_harness::{draw, isolate_in, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

const MANIFEST: &str = r#"[probe]
id = "acme-tools"
name = "Acme tools"
description = "d"
author = "a"
version = "0.1.0"
wormhole = ">=1.3"
api = 1

[permissions]
notify = true

[[contributes.rules]]
id = "failed"
pattern = 'FAILED [a-z_]+'
tone = "danger"

[[contributes.status]]
id = "build"

[[contributes.hooks]]
id = "rang"
on = "bell"
do = [{ type = "status", item = "build", text = "rang", tone = "warning", tooltip = "the bell" }]
"#;

const RUNNER: &str = r#"[probe]
id = "runner"
name = "Runner"
description = "d"
author = "a"
version = "0.1.0"
wormhole = ">=1.3"
api = 1

[permissions]
process = ["./show.sh"]
terminal = ["read"]

[[contributes.status]]
id = "failure"

[[contributes.commands]]
id = "show"
title = "Show"
run = ["./show.sh", "$1"]
in = "background"
cwd = "probe"

[[contributes.hooks]]
id = "failed"
on = "output.match"
match = { pattern = '(?-u:\b)FAILED ([a-z_]{1,40})' }
do = [{ type = "run", command = "show", item = "failure" }]
"#;

/// Makes the read-only copies deletable again, so the harness can remove the scratch folder.
struct Tidy(DataRoot);

impl Drop for Tidy {
    fn drop(&mut self) {
        let _ = approval::sweep(&self.0, Some(&Default::default()));
    }
}

fn install_and_approve(root: &DataRoot) {
    let dir = root.probes_dir().join("acme-tools");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("probe.toml"), MANIFEST).expect("manifest");
    let request = approval::read_one(root, "acme-tools").expect("a request");
    let confirmed = request.confirm(&request.initial_grants(), Proof::TouchId).expect("a yes");
    approval::approve(root, &request, &confirmed, 1_800_000_000).expect("approves");
}

/// Settle what is in motion until `done`, with the real clock (the hooks' thread is a real one).
fn wait_for(cx: &mut TestAppContext, what: &str, mut done: impl FnMut(&mut TestAppContext) -> bool) {
    let until = Instant::now() + Duration::from_secs(10);
    cx.run_until_parked();
    while !done(cx) {
        assert!(Instant::now() < until, "{what}");
        std::thread::sleep(Duration::from_millis(10));
        cx.run_until_parked();
    }
}

fn set_flag(cx: &mut TestAppContext, on: bool) {
    cx.update(|cx| {
        update_settings(cx, |settings| {
            settings.feature_flags.insert("probes".into(), on);
        })
    });
}

#[gpui_kit::test]
fn an_approved_probe_comes_into_force_and_a_bell_reaches_its_hook(cx: &mut TestAppContext) {
    if isolate("an_approved_probe_comes_into_force_and_a_bell_reaches_its_hook") {
        return;
    }
    let window = start(cx, 1);
    let bus = cx.update(|cx| {
        let bus = wormhole_core::ipc::bus::Bus::new();
        crate::automation::install(bus.clone(), cx);
        bus
    });
    set_flag(cx, true);
    let root = DataRoot::current();
    let _tidy = Tidy(root.clone());
    cx.update(crate::app::probes::follow);
    cx.run_until_parked();
    assert!(cx.update(|cx| probe_runtime::rules(cx)).is_none(), "nothing is approved yet");
    assert_eq!(bus.subscribers(), 0, "and the bus is not asked for anything");

    install_and_approve(&root);
    cx.update(probe_runtime::approvals_changed);
    wait_for(cx, "the Probe loads", |cx| cx.update(|cx| probe_runtime::rules(cx)).is_some());
    assert_eq!(cx.update(|cx| probe_runtime::probe_name("acme-tools", cx)).as_deref(), Some("Acme tools"));
    assert_eq!(bus.subscribers(), 1, "one subscription, for the bell");

    // A bell on the bus is the Probe's hook: its status item, in the Probe's own words.
    let term = terminal(cx, &window, 0);
    event(cx, &term, PtyEvent::Bell);
    wait_for(cx, "the hook sets its item", |cx| !cx.update(|cx| probe_runtime::status_items(cx)).is_empty());
    let items = cx.update(|cx| probe_runtime::status_items(cx));
    assert_eq!(items.len(), 1);
    assert_eq!((items[0].text.as_str(), items[0].probe_name.as_str()), ("rang", "Acme tools"));
    assert_eq!(items[0].tooltip.as_deref(), Some("the bell"));
    draw(cx, &window);

    // Turned off, everything it did is gone and the bus has no listener of ours.
    approval::turn_off(&root, "acme-tools").expect("off");
    cx.update(probe_runtime::approvals_changed);
    wait_for(cx, "the Probe is dropped", |cx| cx.update(|cx| probe_runtime::rules(cx)).is_none());
    assert!(cx.update(|cx| probe_runtime::status_items(cx)).is_empty());
    wait_for(cx, "the feed ends", |_| bus.subscribers() == 0);

    // The flag off: nothing runs and nothing listens.
    install_and_approve(&root);
    cx.update(probe_runtime::approvals_changed);
    wait_for(cx, "the Probe loads again", |cx| cx.update(|cx| probe_runtime::rules(cx)).is_some());
    set_flag(cx, false);
    wait_for(cx, "the flag turns it off", |cx| cx.update(|cx| probe_runtime::rules(cx)).is_none());
    wait_for(cx, "the feed ends", |_| bus.subscribers() == 0);
}

/// A line a terminal prints is matched by a hook of a Probe that may read the terminals, the Probe's program
/// is started with the capture as its one argument, and what it says becomes the status item. Everything in
/// between is real: the PTY's output, the bus's worker, the approved copy and the spawn.
#[gpui_kit::test]
fn a_printed_line_runs_the_probe_s_program_and_its_answer_is_an_item(cx: &mut TestAppContext) {
    if isolate("a_printed_line_runs_the_probe_s_program_and_its_answer_is_an_item") {
        return;
    }
    let window = start(cx, 1);
    let bus = cx.update(|cx| {
        let bus = wormhole_core::ipc::bus::Bus::new();
        crate::automation::install(bus.clone(), cx);
        bus
    });
    set_flag(cx, true);
    let root = DataRoot::current();
    let _tidy = Tidy(root.clone());
    let dir = root.probes_dir().join("runner");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("probe.toml"), RUNNER).expect("manifest");
    let script = dir.join("show.sh");
    fs::write(&script, "#!/bin/sh\necho \"failed: $1\"\n").expect("script");
    fs::set_permissions(&script, wormhole_core::platform::fs::permissions(0o755)).expect("chmod");
    let request = approval::read_one(&root, "runner").expect("a request");
    let confirmed = request.confirm(&request.initial_grants(), Proof::TouchId).expect("a yes");
    approval::approve(&root, &request, &confirmed, 1_800_000_000).expect("approves");

    cx.update(crate::app::probes::follow);
    wait_for(cx, "the feed listens", |_| bus.subscribers() == 1);
    let t = terminal(cx, &window, 0);
    cx.update(|cx| t.update(cx, |t, _| t.write_input(b"build FAILED unit_tests\r".to_vec())));
    wait_for(cx, "the program's answer", |cx| !cx.update(|cx| probe_runtime::status_items(cx)).is_empty());
    let items = cx.update(|cx| probe_runtime::status_items(cx));
    assert_eq!((items[0].item.as_str(), items[0].text.as_str()), ("failure", "failed: unit_tests"));
}

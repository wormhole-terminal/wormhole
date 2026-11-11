//! Probes in the Swift app, over the real server and terminals: a Probe approved through the desk (the owner's
//! authentication answered by a test double, never a prompt), its rules decorating a real terminal's frame, a hook's
//! status item, a fold marker in the frame, a rule's link, and the guards on what may be folded. Everything in a temp
//! data folder of the experiment's own.

use std::fs;
use std::sync::Arc;

use serde_json::json;
use wormhole_core::storage::paths::DataRoot;

use super::support::{Fixture, eventually, fixture};
use crate::files_panel::{OwnerAnswer, OwnerAuthenticator};
use crate::hook_server::HookEvent;
use crate::probes::test_lock;
use crate::settings::tests::write_file;
use crate::{ProbeApprovalOutcome, ProbesDesk};

/// Answers the owner's question with yes: the seam `WORMHOLE_AUTH=allow` is for the app.
struct Allow;
impl OwnerAuthenticator for Allow {
    fn authenticate(&self, _reason: String) -> OwnerAnswer {
        OwnerAnswer::Approved
    }
}

const PROBE: &str = r#"
[probe]
id = "demo"
name = "Demo"
description = "A scratch Probe"
author = "test"
version = "0.1.0"
wormhole = ">=1.3"
api = 1

[permissions]
fold = true

[[contributes.rules]]
id = "failed"
pattern = 'FAILED [a-z_]+'
tone = "danger"

[[contributes.rules]]
id = "mail"
pattern = '[a-z]+@[a-z.]+'
look = "shorten"
keep = "start:1,domain"

[[contributes.rules]]
id = "toml"
pattern = '[A-Za-z]+\.toml:[0-9]+'
hover = "Open {0}"
action = { type = "open-path" }

[[contributes.status]]
id = "rang"

[[contributes.hooks]]
id = "bell"
on = "bell"
do = [{ type = "status", item = "rang", text = "rang", tone = "warning" }]

[[contributes.hooks]]
id = "tidy"
on = "command.finished"
match = { exit = "0" }
do = [{ type = "fold", target = "block-output", label = "build output" }]
"#;

/// The words of `frame` from the sections after the rows and the (empty) list of placements; no cell extras here.
fn sections(frame: &[u8], cols: usize) -> (u32, Vec<u32>) {
    let words: Vec<u32> = frame.chunks(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
    let (flags, changed) = (words[5], words[4] as usize);
    assert_eq!((flags & 4, words[11]), (0, 0), "no extras or placements here");
    let rows = 12 + changed * (2 + cols * 4);
    (flags, words[rows..].to_vec())
}

fn row_text(frame: &[u8], cols: usize, row: u32) -> String {
    let words: Vec<u32> = frame.chunks(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
    let mut at = 12;
    for _ in 0..words[4] {
        if words[at] == row {
            let cells = &words[at + 2..at + 2 + cols * 4];
            return cells.chunks(4).map(|cell| char::from_u32(cell[0]).unwrap_or(' ')).collect();
        }
        at += 2 + cols * 4;
    }
    String::new()
}

/// Turn the flags on, write the Probe, approve it with every permission, and wait until the server has it loaded.
fn probes_on(f: &Fixture) {
    let root = DataRoot::new(&f.dir);
    let config = json!({
        "agent_access": "off", "control_write": true,
        "feature_flags": { "automation": true, "probes": true, "output_folds": true }
    });
    write_file(&root.default_profile().config_file(), &config.to_string());
    fs::create_dir_all(root.probes_dir().join("demo")).expect("probe folder");
    write_file(&root.probes_dir().join("demo/probe.toml"), PROBE);
    let desk = ProbesDesk::at(root, Arc::new(Allow));
    let sheet = desk.sheet("demo".into()).expect("a sheet");
    let all: Vec<u32> = (0..sheet.permissions.len() as u32).collect();
    assert!(desk.needs_touch_id("demo".into(), all.clone()), "a fold is a Touch ID grant");
    assert_eq!(desk.approve("demo".into(), all), ProbeApprovalOutcome::Approved);
    f.server.settings_changed("default".into());
    eventually("the Probe loaded", || {
        f.events().contains(&HookEvent::ProbesChanged) && crate::probes::rules().0.is_some()
    });
}

#[test]
fn a_probes_rules_hooks_and_folds_reach_a_real_terminals_frame() {
    let _only = test_lock();
    let f = fixture("off");
    probes_on(&f);
    let id = f.shell_terminal();
    let terminal = f.workspace.terminal(id).expect("registered");
    let frame = |t: &crate::Terminal| t.frame();
    // The shell works in the scratch folder, which holds the file the rule will name.
    write_file(&f.dir.join("scratch.toml"), "a = 1\n");
    terminal.write(format!("cd '{}'\n", f.dir.display()).into_bytes());
    eventually("the shell in the scratch folder", || {
        terminal.with_pty(|pty| pty.cwd()).flatten().is_some_and(|cwd| cwd.ends_with("Wormhole-swift-hook"))
    });

    // Rules: the failed span is a danger tone, the address is shortened to what the rule keeps.
    terminal.write(b"echo 'FAILED test_a by bob@mail.org see scratch.toml:3'\n".to_vec());
    let mut seen = None;
    eventually("the decorated line", || {
        let bytes = frame(&terminal);
        let (flags, words) = sections(&bytes, 80);
        if flags & 8 == 0 {
            return false;
        }
        seen = Some((bytes, words));
        true
    });
    let (bytes, words) = seen.expect("a frame");
    // count, then the record: row, block tone, span count, then spans of (first, last, tone, mask length, mask).
    let shown = words[1];
    assert!(row_text(&bytes, 80, shown).contains("FAILED test_a"));
    let spans = words[3] as usize;
    assert!(spans >= 2, "the failure and the address: {words:?}");
    let first = &words[4..];
    assert_eq!((first[2], first[3]), (2, 0), "danger is the second tone and the text stays");
    let mask = &first[4 + 4..4 + 4 + 12.min(first[4 + 3] as usize)];
    assert_eq!(first[4 + 2], 0, "no tone on the address");
    let shortened: String = mask.iter().filter_map(|c| char::from_u32(*c)).collect();
    assert!(shortened.starts_with("b…@mail.org"), "{shortened:?}");

    // The pointer over the address shows the real text: the next frame carries no mask for it.
    let (row, col) = (shown, first[4] + 1);
    assert!(terminal.set_reveal(Some(row), Some(col), false), "the revealed span changed");
    let (_, revealed) = sections(&frame(&terminal), 80);
    assert_eq!(revealed[3], 1, "only the failure is still decorated");
    assert!(!terminal.set_reveal(Some(row), Some(col), false), "the same cell asks nothing again");
    assert!(terminal.set_reveal(None, None, false));

    // A rule's file link: not an address, with the Probe named on its readout; the file is found in the folder.
    let link_col = row_text(&bytes, 80, shown).find("scratch.toml").expect("the file") as u32 + 2;
    let link = terminal.link_at(shown, link_col).expect("a rule's link");
    assert!(link.file && link.uri == "scratch.toml:3", "{link:?}");
    assert_eq!(link.readout.as_deref(), Some("Open scratch.toml:3 → scratch.toml:3 · Demo"));
    let found = terminal.resolve_path(link.uri);
    assert!(found.path.is_some_and(|p| p.ends_with("scratch.toml")) && found.line == Some(3));
    assert!(terminal.resolve_path("../../../../etc/passwd".into()).words.is_some(), "out of the folder");

    // A hook's status item: the bell sets it, in the Probe's name.
    terminal.write(b"printf '\\a'\n".to_vec());
    eventually("the status item", || !f.server.probe_status_items().is_empty());
    let item = f.server.probe_status_items().remove(0);
    assert_eq!((item.text.as_str(), item.token.as_deref()), ("rang", Some("status.warning.text")));
    assert_eq!(item.tooltip, "Demo");
}

#[test]
fn a_finished_blocks_output_folds_to_a_marker_that_opens_and_that_never_hide_lines_takes_away() {
    let _only = test_lock();
    let f = fixture("off");
    probes_on(&f);
    let id = f.shell_terminal();
    let terminal = f.workspace.terminal(id).expect("registered");
    terminal.write(b"seq 1 60\n".to_vec());
    // The fold hides rows of the history: the view shows its marker once it is scrolled up to them.
    eventually("the fold", || terminal.has_folds());
    terminal.scroll(40);
    let (flags, words) = sections(&terminal.frame(), 80);
    assert_eq!((flags & 16, flags & 8), (16, 0), "a marker and no decorations: {flags}");

    // count, row, id lo, id hi, hidden, flags, then the strings.
    assert_eq!(words[0], 1);
    let (fold_id, hidden) = (u64::from(words[2]) | u64::from(words[3]) << 32, words[4]);
    assert!(hidden >= 3 && words[5] == 0, "closed, not failed: {words:?}");
    let label_len = words[6] as usize;
    let text = |at: usize, len: usize| -> String {
        let bytes: Vec<u8> = words[at..].iter().flat_map(|w| w.to_le_bytes()).take(len).collect();
        String::from_utf8(bytes).expect("utf-8")
    };
    assert!(text(7, label_len).ends_with("· build output"), "{}", text(7, label_len));
    assert!(terminal.has_folds());

    // While a search is open the marker counts the matches inside the lines it hides.
    let found = terminal.find_search("^[0-9]+$".into(), false, false, true);
    assert!(found.count >= hidden, "every hidden line is a number: {found:?}");
    let _ = terminal.frame();
    let counted = terminal.find_fold_words();
    assert_eq!(counted.len(), 1, "the one marker in view: {counted:?}");
    assert!(
        counted[0].label.contains("matches inside") && counted[0].spoken.contains("matches inside"),
        "{counted:?}"
    );

    // Opening shows the lines and says how many; closing again is the marker's own click.
    assert_eq!(terminal.fold_show_all(), hidden);
    // An open fold keeps its marker, above its own lines.
    terminal.scroll(200);
    let (flags, open) = sections(&terminal.frame(), 80);
    assert_eq!(flags & 16, 16, "an open fold has a marker row: {flags}");
    assert_eq!(open[5] & 1, 1, "and says it is open: {open:?}");
    assert!(terminal.fold_toggle(fold_id), "closing it again is the marker's own click");
    // Stepping to a match inside the closed fold opens it, as the GPUI app's find does.
    terminal.find_reveal(found.count / 2);
    terminal.scroll(200);
    let (_, shown) = sections(&terminal.frame(), 80);
    assert_eq!(shown[5] & 1, 1, "the match's fold is open: {shown:?}");
    assert!(terminal.fold_toggle(fold_id));

    // "Never hide lines": the view shows every line.
    let root = DataRoot::new(&f.dir);
    let config = json!({
        "agent_access": "off", "never_hide_lines": true,
        "feature_flags": { "automation": true, "probes": true, "output_folds": true }
    });
    write_file(&root.default_profile().config_file(), &config.to_string());
    f.server.settings_changed("default".into());
    eventually("every line shown", || {
        let (flags, _) = sections(&terminal.frame(), 80);
        flags & 16 == 0
    });
}

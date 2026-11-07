//! A Probe's fold through a real window's terminal: it is made under the rules, says whose it is, yields to
//! the person's "never hide lines" and goes when the Probe does. The shell's marks are fed to the terminal's
//! screen as its PTY thread would (`/bin/cat` prints none), in a child process with a scratch data dir
//! (`isolate_in`).

use gpui_kit::TestAppContext;
use wormhole_term::folds::{FoldOwner, FoldSpan};
use wormhole_term::types::Scroll;
use wormhole_term::{Pty, ScreenSnapshot};

use crate::settings::update_settings;
use crate::window::reported_tests::terminal;
use crate::window::test_harness::{isolate_in, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

fn acme() -> FoldOwner {
    FoldOwner { probe: "acme-tools".into(), name: "Acme tools".into(), label: "build output".into() }
}

/// Two finished commands with `lines` lines of output each: the first one's output has scrolled into the
/// history. Returns the first command's block, and shows the top of the history.
fn two_commands(pty: &Pty, lines: usize) -> u64 {
    let mut prompts = 0;
    let output: String = (0..lines).map(|n| format!("line {n}\r\n")).collect();
    for command in ["one", "two"] {
        let bytes =
            format!("\x1b]133;A\x07$ \x1b]133;B\x07{command}\r\n\x1b]133;C\x07{output}\x1b]133;D;0\x07");
        pty.reader().print_marked(bytes.as_bytes(), &mut prompts);
    }
    pty.reader().print_marked(b"\x1b]133;A\x07$ \x1b]133;B\x07", &mut prompts);
    pty.scroll(Scroll::Top);
    pty.blocks().first().expect("a block").id
}

/// The markers the terminal's view has now.
fn markers(
    cx: &mut TestAppContext,
    term: &gpui_kit::Entity<crate::terminal_view::TerminalView>,
) -> Vec<FoldSpan> {
    term.read_with(cx, |t, _| {
        let mut snapshot = ScreenSnapshot::default();
        t.pty_for_test().expect("a pty").snapshot(&mut snapshot);
        snapshot.rows().iter().filter_map(|row| row.fold().cloned()).collect()
    })
}

#[gpui_kit::test]
fn a_probes_fold_names_the_probe_and_goes_with_it(cx: &mut TestAppContext) {
    if isolate("a_probes_fold_names_the_probe_and_goes_with_it") {
        return;
    }
    let window = start(cx, 1);
    cx.update(|cx| {
        update_settings(cx, |settings| {
            settings.feature_flags.insert("output_folds".into(), true);
        })
    });
    let term = terminal(cx, &window, 0);
    let block = term.read_with(cx, |t, _| t.pty_for_test().map(|pty| two_commands(pty, 60))).expect("a pty");
    cx.update(|cx| term.update(cx, |t, cx| t.fold_for_probe(block, acme(), (1, 2), cx)));
    let found = markers(cx, &term);
    let [fold] = &found[..] else { panic!("one marker: {found:?}") };
    assert_eq!(fold.hidden(), 57, "the output less the head and the tail it kept");
    assert_eq!(fold.owner.as_deref().map(|owner| owner.name.as_str()), Some("Acme tools"));

    // "Never hide lines" is the person's: the view shows every line while it is on, and the fold waits.
    cx.update(|cx| update_settings(cx, |settings| settings.never_hide_lines = true));
    cx.run_until_parked();
    assert!(markers(cx, &term).is_empty());
    cx.update(|cx| update_settings(cx, |settings| settings.never_hide_lines = false));
    cx.run_until_parked();
    assert_eq!(markers(cx, &term).len(), 1);

    // The Probe no longer may fold (turned off, or the grant taken away): its fold is gone at once.
    cx.update(|cx| term.update(cx, |t, cx| t.retain_probe_folds(&["other".to_string()], cx)));
    assert!(markers(cx, &term).is_empty());
}

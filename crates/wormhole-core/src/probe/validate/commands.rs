//! `[[contributes.commands]]` and `[[contributes.status]]`: what a Probe runs
//! (sections 3.2 and 3.3 of the design).

use super::Findings;
use super::argv::{self, Where};
use crate::keys::keymap;
use crate::keys::keyspec::KeySpec;
use crate::probe::hidden::has_hidden;
use crate::probe::manifest::{Command, RunIn, StatusItem};
use crate::probe::read::ProbeFolder;

/// Seconds between two runs of a status item, at least, and when none is given.
pub(crate) const MIN_INTERVAL: u64 = 5;
pub(crate) const DEFAULT_INTERVAL: u64 = 30;
/// Seconds before a status item's process is killed, at most.
pub(crate) const MAX_TIMEOUT: u64 = 10;

pub(super) fn check(folder: &ProbeFolder, out: &mut Findings) {
    let c = &folder.manifest.contributes;
    let has_code = folder.manifest.probe.main.is_some();
    let mut keys: Vec<(usize, KeySpec)> = Vec::new();
    for (ix, command) in c.commands.iter().enumerate() {
        check_command(folder, command, ix, has_code, out);
        if let Some(key) = check_key(command, ix, out) {
            if let Some((other, _)) = keys.iter().find(|(_, taken)| taken.same_key(&key)) {
                out.error(
                    "command-key-duplicate",
                    format!("contributes.commands[{ix}].key"),
                    format!("commands[{other}] already has this key"),
                );
            }
            keys.push((ix, key));
        }
    }
    for (ix, item) in c.status.iter().enumerate() {
        check_status(folder, item, ix, has_code, out);
    }
}

fn check_command(folder: &ProbeFolder, command: &Command, ix: usize, has_code: bool, out: &mut Findings) {
    let at = format!("contributes.commands[{ix}]");
    if command.title.trim().is_empty() {
        out.error("command-title", format!("{at}.title"), "must not be empty");
    } else if has_hidden(&command.title) {
        out.error(
            "command-title",
            format!("{at}.title"),
            "must not hold control, bidi or invisible characters",
        );
    }
    if let Some(description) = &command.description
        && has_hidden(description)
    {
        out.error(
            "command-title",
            format!("{at}.description"),
            "must not hold control, bidi or invisible characters",
        );
    }
    let Some(run) = &command.run else {
        if !has_code {
            out.error(
                "command-no-run",
                &at,
                "has no `run`, so only code could handle it, and this Probe has no `main` (code is E2, not supported yet)",
            );
        }
        return;
    };
    let path = format!("{at}.run");
    argv::check(
        folder,
        run,
        &argv::Where {
            path: &path,
            new_terminal: command.run_in == RunIn::NewTerminal,
            captures_allowed: true,
        },
        out,
    );
    let captures = argv::captures_of(run);
    if captures.is_empty() {
        return;
    }
    if command.run_in == RunIn::NewTerminal {
        out.error(
            "command-capture-new-terminal",
            &path,
            "a capture would reach the user's own shell in a new terminal, which is not safe until captures are quoted for it; use `in = \"background\"`",
        );
    }
    let targeted =
        folder.manifest.contributes.rules.iter().any(|rule| {
            rule.action.as_ref().is_some_and(|a| a.command.as_deref() == Some(command.id.as_str()))
        });
    if !targeted {
        out.warning(
            "command-capture-unbound",
            &path,
            "uses `$n`, but no rule's action runs this command, so from the palette nothing fills it",
        );
    }
}

/// A command's shortcut, when it has one and it is a key: it must parse, be a real key, and be free
/// the way a project action's is (not the app's, not macOS's, not the shell's).
fn check_key(command: &Command, ix: usize, out: &mut Findings) -> Option<KeySpec> {
    let text = command.key.as_deref()?;
    let path = format!("contributes.commands[{ix}].key");
    let key = match text.parse::<KeySpec>() {
        Ok(key) if key.has_known_key() => key,
        Ok(key) => {
            out.error("command-key", path, format!("`{}` is not a key", key.key));
            return None;
        }
        Err(err) => {
            out.error("command-key", path, err.to_string());
            return None;
        }
    };
    if let Some(taken) = keymap::uses(&key) {
        out.error("command-key-taken", path, format!("{}: {taken}", key.glyphs()));
        return None;
    }
    Some(key)
}

fn check_status(folder: &ProbeFolder, item: &StatusItem, ix: usize, has_code: bool, out: &mut Findings) {
    let at = format!("contributes.status[{ix}]");
    match &item.run {
        None if !has_code => out.error(
            "status-no-run",
            &at,
            "has no `run`, so only code could feed it, and this Probe has no `main` (code is E2, not supported yet)",
        ),
        None => {}
        Some(run) => argv::check(
            folder,
            run,
            &Where { path: &format!("{at}.run"), new_terminal: false, captures_allowed: false },
            out,
        ),
    }
    if let Some(interval) = item.interval
        && interval < MIN_INTERVAL
    {
        out.error(
            "status-interval",
            format!("{at}.interval"),
            format!("{interval} s is too often: at least {MIN_INTERVAL} s (it is {DEFAULT_INTERVAL} s when not given)"),
        );
    }
    if let Some(timeout) = item.timeout
        && !(1..=MAX_TIMEOUT).contains(&timeout)
    {
        out.error(
            "status-timeout",
            format!("{at}.timeout"),
            format!("{timeout} s is out of range: 1 to {MAX_TIMEOUT} s"),
        );
    }
}

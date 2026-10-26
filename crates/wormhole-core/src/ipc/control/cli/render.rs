//! The app's reply as text for a person.

use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::ipc::control::{
    Action, Asked, Listing, Opened, Reached, Reply, StatusInfo, Summary, TabInfo, TerminalInfo,
};

/// The reply to `action` as a person reads it (`--json` prints the object instead). The action
/// says what the result is: a result is read as the type that action returns, never guessed from
/// its fields. `home` shortens folders under it to `~`. An error comes back as `Err` with its
/// words, for stderr.
pub fn render(action: &Action, reply: &Reply, home: Option<&Path>) -> Result<String, String> {
    if let Some(error) = &reply.error {
        let kind = match error.reason {
            Some(reason) => format!("{}, {}", name_of(error.code), name_of(reason)),
            None => name_of(error.code),
        };
        return Err(format!("{} ({kind})", error.message));
    }
    let result = reply.result.clone().unwrap_or(Value::Null);
    let text = match action {
        Action::List => read::<Listing>(&result).map(|listing| listing_text(&listing, home)),
        Action::Status { .. } => read::<StatusInfo>(&result)
            .map(|status| status_text(&status, home))
            .or_else(|| read::<Summary>(&result).map(|summary| summary_text(&summary))),
        Action::NewTerminal { .. } => read::<Opened>(&result)
            .map(|opened| opened_text(&opened, home))
            .or_else(|| read::<Asked>(&result).map(|asked| asked_text(&asked, home))),
        Action::Notify { .. } | Action::SendText { .. } => {
            read::<Reached>(&result).map(|reached| format!("tab {}\n", reached.tab))
        }
        // Streams and the welcome have their own words.
        Action::Events(_) | Action::Welcome { .. } => None,
    };
    Ok(text.unwrap_or_else(|| "ok\n".into()))
}

/// `result` as the `T` its action returns, when it is one.
fn read<T: DeserializeOwned>(result: &Value) -> Option<T> {
    serde_json::from_value(result.clone()).ok()
}

fn listing_text(listing: &Listing, home: Option<&Path>) -> String {
    let mut out = String::new();
    for window in &listing.windows {
        out.push_str(&format!("window {}{}\n", window.id, if window.front { "  front" } else { "" }));
        for terminal in &window.terminals {
            out.push_str(&terminal_line(terminal, home));
            for tab in &terminal.tabs {
                out.push_str(&tab_line(tab));
            }
        }
    }
    if listing.windows.is_empty() {
        out.push_str("no windows\n");
    }
    out
}

fn status_text(status: &StatusInfo, home: Option<&Path>) -> String {
    let mut out = format!("window {}\n", status.window);
    out.push_str(&terminal_line(&status.terminal, home));
    for tab in status.terminal.tabs.iter().filter(|tab| tab.id == status.tab) {
        out.push_str(&tab_line(tab));
    }
    out
}

fn summary_text(summary: &Summary) -> String {
    format!(
        "{} {}, {} {}: {} needs you, {} working, {} finished, {} failed, {} idle, {} exited\n",
        summary.windows,
        plural(summary.windows, "window"),
        summary.terminals,
        plural(summary.terminals, "terminal"),
        summary.needs_you,
        summary.working,
        summary.finished,
        summary.failed,
        summary.idle,
        summary.exited
    )
}

fn opened_text(opened: &Opened, home: Option<&Path>) -> String {
    let folder = opened.folder.as_deref().map(|f| shorten(f, home)).unwrap_or_default();
    let worktree = opened.worktree.as_deref().map(|w| format!(" on worktree {w}")).unwrap_or_default();
    format!(
        "opened terminal {} (tab {}, pane {}) in window {} at {folder}{worktree}\n",
        opened.terminal, opened.tab, opened.pane, opened.window
    )
}

fn asked_text(asked: &Asked, home: Option<&Path>) -> String {
    format!(
        "asked in window {}: confirm the worktree {} at {} there; the terminal opens once you do\n",
        asked.window,
        asked.branch,
        shorten(&asked.folder, home)
    )
}

fn terminal_line(terminal: &TerminalInfo, home: Option<&Path>) -> String {
    let mut parts =
        vec![format!("  terminal {}  {:?}", terminal.id, terminal.name), terminal.status.word().to_string()];
    if let Some(agent) = &terminal.agent {
        parts.push(agent.clone());
    }
    if let Some(folder) = &terminal.folder {
        parts.push(shorten(folder, home));
    }
    if let Some(branch) = &terminal.branch {
        match terminal.changed {
            Some(changed) if changed > 0 => parts.push(format!("{branch} ±{changed}")),
            _ => parts.push(branch.clone()),
        }
    }
    if terminal.pinned {
        parts.push("pinned".into());
    }
    if let Some(profile) = &terminal.profile {
        parts.push(format!("profile {profile}"));
    }
    if terminal.selected {
        parts.push("selected".into());
    }
    format!("{}\n", parts.join("  "))
}

fn tab_line(tab: &TabInfo) -> String {
    let mut parts =
        vec![format!("    tab {}  {:?}", tab.id, tab.name), tab.status.word().to_string(), tab.shell.clone()];
    if let Some(command) = &tab.command {
        parts.push(format!("running {command}"));
    }
    if let Some(message) = &tab.message {
        parts.push(format!("{message:?}"));
    }
    if tab.selected {
        parts.push("selected".into());
    }
    let panes: Vec<String> = tab.panes.iter().map(|pane| pane.id.to_string()).collect();
    parts.push(format!("{} {}", plural(panes.len(), "pane"), panes.join(" ")));
    format!("{}\n", parts.join("  "))
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 { word.to_string() } else { format!("{word}s") }
}

/// `folder` with `home` as `~`.
pub(super) fn shorten(folder: &str, home: Option<&Path>) -> String {
    match home.map(|home| home.to_string_lossy().into_owned()) {
        Some(home) if folder == home => "~".into(),
        Some(home) if folder.starts_with(&format!("{home}/")) => format!("~{}", &folder[home.len()..]),
        _ => folder.to_string(),
    }
}

/// A code or reason as the text output names it (`not_allowed`).
fn name_of(word: impl Serialize) -> String {
    serde_json::to_value(word).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{render, shorten};
    use crate::ipc::control::{
        Action, Error, ErrorCode, Listing, Opened, PaneInfo, Reached, Reason, Reply, StatusInfo, Summary,
        TabInfo, TerminalInfo, TerminalState, WindowInfo,
    };

    fn status_of() -> Action {
        Action::Status { target: None }
    }

    fn new_terminal() -> Action {
        Action::NewTerminal { folder: None, command: None, worktree: None, window: None }
    }

    fn notify() -> Action {
        Action::Notify { target: None, title: String::new(), body: "x".into() }
    }

    fn tab(id: i64, name: &str, status: TerminalState, selected: bool) -> TabInfo {
        TabInfo {
            id,
            name: name.into(),
            status,
            agent: None,
            message: None,
            folder: Some("/Users/me/src/api".into()),
            shell: "zsh".into(),
            command: None,
            since_seconds: 12,
            selected,
            panes: vec![PaneInfo { id }],
        }
    }

    fn terminal() -> TerminalInfo {
        TerminalInfo {
            id: 3,
            name: "api".into(),
            status: TerminalState::NeedsYou,
            agent: Some("claude".into()),
            folder: Some("/Users/me/src/api".into()),
            branch: Some("main".into()),
            changed: Some(2),
            pinned: true,
            profile: None,
            selected: true,
            tabs: vec![
                TabInfo {
                    message: Some("Allow Bash?".into()),
                    agent: Some("claude".into()),
                    ..tab(3, "claude", TerminalState::NeedsYou, true)
                },
                TabInfo {
                    command: Some("npm run dev".into()),
                    ..tab(5, "npm run dev", TerminalState::Working, false)
                },
            ],
        }
    }

    #[test]
    fn a_listing_reads_as_an_indented_tree() {
        let listing =
            Listing { windows: vec![WindowInfo { id: 1, front: true, terminals: vec![terminal()] }] };
        let text = render(&Action::List, &Reply::ok(listing), Some(Path::new("/Users/me"))).expect("renders");
        assert_eq!(
            text,
            "window 1  front\n\
             \x20 terminal 3  \"api\"  needs you  claude  ~/src/api  main ±2  pinned  selected\n\
             \x20   tab 3  \"claude\"  needs you  zsh  \"Allow Bash?\"  selected  pane 3\n\
             \x20   tab 5  \"npm run dev\"  working  zsh  running npm run dev  pane 5\n"
        );
        let empty = render(&Action::List, &Reply::ok(Listing::default()), None).expect("renders");
        assert_eq!(empty, "no windows\n");
    }

    #[test]
    fn the_other_results_read_as_a_line_each() {
        let home = Some(Path::new("/Users/me"));
        let status = StatusInfo { window: 1, terminal: terminal(), tab: 5 };
        let text = render(&status_of(), &Reply::ok(status), home).expect("renders");
        assert!(text.starts_with("window 1\n  terminal 3  \"api\""), "{text}");
        assert!(text.contains("tab 5") && !text.contains("tab 3  "), "only the tab asked about: {text}");
        let summary = Summary::of(2, [TerminalState::Working, TerminalState::Idle, TerminalState::NeedsYou]);
        assert_eq!(
            render(&status_of(), &Reply::ok(summary), home).expect("renders"),
            "2 windows, 3 terminals: 1 needs you, 1 working, 0 finished, 0 failed, 1 idle, 0 exited\n"
        );

        let one = Summary::of(1, [TerminalState::Idle]);
        assert!(
            render(&status_of(), &Reply::ok(one), home)
                .expect("renders")
                .starts_with("1 window, 1 terminal:")
        );
        let opened = Opened {
            window: 1,
            terminal: 8,
            tab: 8,
            pane: 8,
            folder: Some("/Users/me/src/api".into()),
            worktree: Some("fix/login".into()),
        };
        assert_eq!(
            render(&new_terminal(), &Reply::ok(opened), home).expect("renders"),
            "opened terminal 8 (tab 8, pane 8) in window 1 at ~/src/api on worktree fix/login\n"
        );
        assert_eq!(render(&notify(), &Reply::ok(Reached { tab: 4 }), home).expect("renders"), "tab 4\n");
        let error = render(&notify(), &Reply::error(ErrorCode::NotAllowed, "typing is off"), home);
        assert_eq!(error, Err("typing is off (not_allowed)".into()));
    }

    #[test]
    fn home_is_shortened_to_a_tilde() {
        let home = Some(Path::new("/Users/me"));
        assert_eq!(shorten("/Users/me", home), "~");
        assert_eq!(shorten("/Users/me/src", home), "~/src");
        assert_eq!(shorten("/Users/meh/src", home), "/Users/meh/src", "not a prefix of a longer name");
        assert_eq!(shorten("/tmp", home), "/tmp");
        assert_eq!(shorten("/Users/me/src", None), "/Users/me/src");
    }

    /// The result is read as what the action returns: a `{"tab": …}` is `tab 4` for a `notify`,
    /// not for whichever request happened to be asked.
    #[test]
    fn a_result_is_read_as_what_the_action_returns() {
        let reached = Reply::ok(Reached { tab: 4 });
        assert_eq!(render(&notify(), &reached, None).expect("renders"), "tab 4\n");
        assert_eq!(render(&Action::List, &reached, None).expect("renders"), "ok\n");
        assert_eq!(render(&status_of(), &reached, None).expect("renders"), "ok\n");
    }

    #[test]
    fn an_errors_reason_is_named_beside_its_code() {
        let error = Reply::from(Error::new(ErrorCode::Failed, "took too long").because(Reason::Timeout));
        assert_eq!(render(&notify(), &error, None), Err("took too long (failed, timeout)".into()));
    }
}

//! What a notification and an ambient row say: the title names the
//! terminal and what happened, the subtitle who and where, the body the agent's words or how long
//! it took; and the times of the menu rows ("2m · 4:12 · 12m ago").

use std::time::Duration;

use super::{Kind, State};
use crate::sidebar::text::{ago, clock, took};

/// The facts a notification is written from.
#[derive(Clone, Copy, Debug, Default)]
pub struct About<'a> {
    /// The terminal's name, as the sidebar shows it.
    pub name: &'a str,
    /// The agent's name ("Claude Code"), if one runs there.
    pub agent: Option<&'a str>,
    /// The command that ran ("cargo build --release"), for a terminal without an agent.
    pub command: Option<&'a str>,
    /// The folder's name and the branch.
    pub folder: Option<&'a str>,
    pub branch: Option<&'a str>,
    /// The agent's question, or the body of a program's own notification.
    pub message: Option<&'a str>,
    /// The title a program gave its notification.
    pub title: Option<&'a str>,
    /// How long the agent worked, or the command ran.
    pub took: Option<Duration>,
    /// Files changed in the terminal's repository, when known.
    pub changed: Option<usize>,
}

/// A notification's three lines.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Text {
    pub title: String,
    pub subtitle: String,
    pub body: String,
}

/// The words of a notification.
pub fn text(kind: Kind, about: &About) -> Text {
    let name = if about.name.trim().is_empty() { "Terminal" } else { about.name.trim() };
    match kind {
        Kind::NeedsYou => Text {
            title: format!("{name} needs you"),
            subtitle: joined([about.agent, about.folder, about.branch]),
            body: words(about.message).unwrap_or("Waiting for your answer").to_string(),
        },
        Kind::AgentDone => {
            let changed = about.changed.filter(|&n| n > 0).map(files_changed);
            let body = joined([about.took.map(duration).as_deref(), changed.as_deref()]);
            Text {
                title: format!("{name} finished"),
                subtitle: joined([about.agent, about.folder, about.branch]),
                body: if body.is_empty() { "Finished".to_string() } else { body },
            }
        }
        Kind::CommandDone => {
            let command = words(about.command).unwrap_or(name);
            Text {
                title: format!("{command} finished"),
                subtitle: joined([Some(name).filter(|n| *n != command), about.folder]),
                body: about.took.map_or_else(|| "Finished".to_string(), duration),
            }
        }
        Kind::Bell => Text {
            title: format!("{name} rang the bell"),
            subtitle: joined([words(about.command), about.folder]),
            body: "Bell while you were in another terminal".to_string(),
        },
        Kind::Notice => {
            // The program's own title and body; with only one of them, the terminal's name is
            // the title and the program's line the body.
            let (title, body) = match (words(about.title), words(about.message)) {
                (Some(title), Some(body)) => (title.to_string(), body.to_string()),
                (Some(line), None) | (None, Some(line)) => (name.to_string(), line.to_string()),
                (None, None) => (name.to_string(), "Needs your attention".to_string()),
            };
            let subtitle = joined([
                Some(name).filter(|n| *n != title),
                about.agent.or(words(about.command)),
                about.folder,
            ]);
            Text { title, subtitle, body }
        }
    }
}

/// The summary that stands for a burst: how many terminals and what they did,
/// then their names.
pub fn summary(kinds: &[Kind], names: &[String]) -> Text {
    let n = names.len();
    let finished = kinds.iter().all(|k| matches!(k, Kind::AgentDone | Kind::CommandDone));
    let title = match () {
        _ if finished => format!("{n} terminals finished"),
        _ if kinds.iter().all(|k| *k == Kind::Bell) => format!("{n} terminals rang the bell"),
        _ => format!("{n} terminals"),
    };
    Text { title, subtitle: "Wormhole".to_string(), body: names.join(", ") }
}

/// How long something took, for a notification's body: "38s", "4m 12s", "1h 5m".
pub fn duration(d: Duration) -> String {
    let secs = d.as_secs();
    match secs {
        0..60 => format!("{secs}s"),
        60..3600 => format!("{}m {}s", secs / 60, secs % 60),
        _ => format!("{}h {}m", secs / 3600, secs % 3600 / 60),
    }
}

/// A row's time in the menu, the Dock menu and the quick panel: how long it has waited
/// ("2m", "now"), how long it has run ("4:12"), or when it ended ("12m ago"). `None` for a
/// terminal that is idle or ended.
pub fn elapsed(state: State, since: Duration) -> Option<String> {
    match state {
        State::NeedsYou => Some(took(since).unwrap_or_else(|| "now".to_string())),
        State::Working => Some(clock(since)),
        State::Done => Some(ago(since)),
        State::Idle | State::Exited => None,
    }
}

fn files_changed(n: usize) -> String {
    if n == 1 { "1 file changed".to_string() } else { format!("{n} files changed") }
}

fn words(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|t| !t.is_empty())
}

/// The parts that are there, with " · " between them.
fn joined<'a>(parts: impl IntoIterator<Item = Option<&'a str>>) -> String {
    parts.into_iter().filter_map(words).collect::<Vec<_>>().join(" · ")
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{About, Text, duration, elapsed, summary, text};
    use crate::notify::{Kind, State};

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn needs_you_names_the_terminal_and_quotes_the_agent() {
        let about = About {
            name: "diff perf",
            agent: Some("Claude Code"),
            folder: Some("terminal"),
            branch: Some("ui-unify"),
            message: Some("Claude wants to run cargo test -p wormhole-core"),
            ..About::default()
        };
        assert_eq!(
            text(Kind::NeedsYou, &about),
            Text {
                title: "diff perf needs you".into(),
                subtitle: "Claude Code · terminal · ui-unify".into(),
                body: "Claude wants to run cargo test -p wormhole-core".into(),
            }
        );
        let quiet = About { message: Some("  "), ..about };
        assert_eq!(text(Kind::NeedsYou, &quiet).body, "Waiting for your answer");
    }

    #[test]
    fn an_agent_finishing_says_how_long_and_what_changed() {
        let about = About {
            name: "sidebar polish",
            agent: Some("Claude Code"),
            folder: Some("terminal"),
            branch: Some("ui-unify"),
            took: Some(secs(252)),
            changed: Some(7),
            ..About::default()
        };
        let t = text(Kind::AgentDone, &about);
        assert_eq!(
            (t.title.as_str(), t.body.as_str()),
            ("sidebar polish finished", "4m 12s · 7 files changed")
        );
        let bare = About { name: "x", ..About::default() };
        assert_eq!(text(Kind::AgentDone, &bare).body, "Finished");
        assert_eq!(text(Kind::AgentDone, &bare).subtitle, "");
    }

    #[test]
    fn a_command_finishing_is_named_by_its_command() {
        let about = About {
            name: "release",
            command: Some("cargo build --release"),
            folder: Some("terminal"),
            took: Some(secs(192)),
            ..About::default()
        };
        assert_eq!(
            text(Kind::CommandDone, &about),
            Text {
                title: "cargo build --release finished".into(),
                subtitle: "release · terminal".into(),
                body: "3m 12s".into(),
            }
        );
        // A terminal named after its command doesn't say it twice.
        let same = About { name: "make", command: Some("make"), ..About::default() };
        assert_eq!(text(Kind::CommandDone, &same).subtitle, "");
    }

    #[test]
    fn a_programs_notification_keeps_its_own_words() {
        let about = About {
            name: "deploy",
            command: Some("./release.sh"),
            folder: Some("terminal"),
            title: Some("Release"),
            message: Some("v1.2 is live"),
            ..About::default()
        };
        assert_eq!(
            text(Kind::Notice, &about),
            Text {
                title: "Release".into(),
                subtitle: "deploy · ./release.sh · terminal".into(),
                body: "v1.2 is live".into()
            }
        );
        // iTerm2's OSC 9 has only a body: the terminal is the title.
        let body_only = About { title: None, ..about };
        let t = text(Kind::Notice, &body_only);
        assert_eq!(
            (t.title.as_str(), t.subtitle.as_str(), t.body.as_str()),
            ("deploy", "./release.sh · terminal", "v1.2 is live")
        );
        let title_only = About { message: None, ..about };
        assert_eq!(text(Kind::Notice, &title_only).body, "Release");
        let agent = About { agent: Some("Claude Code"), ..about };
        assert_eq!(text(Kind::Notice, &agent).subtitle, "deploy · Claude Code · terminal");
        let bare = About { name: "x", ..About::default() };
        assert_eq!(
            text(Kind::Notice, &bare),
            Text { title: "x".into(), subtitle: String::new(), body: "Needs your attention".into() }
        );
    }

    #[test]
    fn the_bell_says_where_it_rang() {
        let about = About {
            name: "prod",
            command: Some("ssh deploy@prod-1"),
            folder: Some("dotfiles"),
            ..About::default()
        };
        let t = text(Kind::Bell, &about);
        assert_eq!(t.title, "prod rang the bell");
        assert_eq!(t.subtitle, "ssh deploy@prod-1 · dotfiles");
    }

    #[test]
    fn an_unnamed_terminal_still_reads() {
        assert_eq!(text(Kind::NeedsYou, &About::default()).title, "Terminal needs you");
    }

    #[test]
    fn a_summary_says_how_many_and_who() {
        let names = ["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(summary(&[Kind::Bell; 3], &names).title, "3 terminals rang the bell");
        let mixed = summary(&[Kind::AgentDone, Kind::Bell, Kind::CommandDone], &names);
        assert_eq!((mixed.title.as_str(), mixed.subtitle.as_str()), ("3 terminals", "Wormhole"));
        assert_eq!(mixed.body, "a, b, c");
    }

    #[test]
    fn durations_read_short() {
        assert_eq!(duration(secs(38)), "38s");
        assert_eq!(duration(secs(192)), "3m 12s");
        assert_eq!(duration(secs(3900)), "1h 5m");
    }

    #[test]
    fn menu_times_wait_run_or_ended() {
        assert_eq!(elapsed(State::NeedsYou, secs(130)).as_deref(), Some("2m"));
        assert_eq!(elapsed(State::NeedsYou, secs(20)).as_deref(), Some("now"));
        assert_eq!(elapsed(State::Working, secs(252)).as_deref(), Some("4:12"));
        assert_eq!(elapsed(State::Done, secs(720)).as_deref(), Some("12m ago"));
        assert_eq!(elapsed(State::Done, secs(5)).as_deref(), Some("just now"));
        assert_eq!(elapsed(State::Idle, secs(5)), None);
        assert_eq!(elapsed(State::Exited, secs(5)), None);
    }
}

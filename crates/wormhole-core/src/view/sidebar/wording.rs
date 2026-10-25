//! The words of a row: what a terminal's second line says and how its times read.

use std::path::Path;
use std::time::Duration;

use super::state::RowState;

/// What the second line of a row whose status something tells (an agent, or a program that
/// reports its own) says, or `None` for a one-line row: "Needs input · Approve cargo test?",
/// "Editing sidebar.rs…" (its own words) or "Working · 3m" ("Working · <1m" in the first minute),
/// "Finished · 12m ago", "Failed · rate limit reached". `since` is how long it has been in
/// `state`. A result with words of its own ("Upgraded 12 packages") says them in place of when.
pub fn agent_detail(state: RowState, message: Option<&str>, since: Duration) -> Option<String> {
    let message = message.map(str::trim).filter(|m| !m.is_empty());
    match state {
        RowState::NeedsYou => Some(match message {
            Some(message) => format!("Needs input · {message}"),
            None => "Needs input".to_string(),
        }),
        RowState::Working => Some(match message {
            Some(message) => message.to_string(),
            None => format!("Working · {}", took(since).as_deref().unwrap_or(UNDER_A_MINUTE)),
        }),
        RowState::Finished => {
            Some(format!("Finished · {}", message.map_or_else(|| ago(since), str::to_string)))
        }
        RowState::Failed => Some(format!("Failed · {}", message.map_or_else(|| ago(since), str::to_string))),
        RowState::Exited => Some("Exited".to_string()),
        RowState::Idle => None,
    }
}

/// "1 terminal", "3 terminals".
pub fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Work that has gone on for less than a minute: times count whole minutes, never seconds.
pub const UNDER_A_MINUTE: &str = "<1m";

/// How long something has taken, short: `None` under a minute, then "3m", "1h", "2d".
pub fn took(elapsed: Duration) -> Option<String> {
    let minutes = elapsed.as_secs() / 60;
    match minutes {
        0 => None,
        1..60 => Some(format!("{minutes}m")),
        60..1440 => Some(format!("{}h", minutes / 60)),
        _ => Some(format!("{}d", minutes / 1440)),
    }
}

/// How long ago, short: "just now", "12m ago", "1h ago", "2d ago".
pub fn ago(elapsed: Duration) -> String {
    took(elapsed).map_or_else(|| "just now".to_string(), |took| format!("{took} ago"))
}

/// A running command's time on the clock: "0:05", "1:12", "1:02:03".
pub fn clock(elapsed: Duration) -> String {
    let secs = elapsed.as_secs();
    let (hours, minutes, seconds) = (secs / 3600, secs / 60 % 60, secs % 60);
    if hours > 0 { format!("{hours}:{minutes:02}:{seconds:02}") } else { format!("{minutes}:{seconds:02}") }
}

/// A path for a narrow line: cut from the front, so the end, which tells two apart, stays
/// (`…/clients/acme/app`). Keeps whole components; `max` counts characters.
pub fn path_tail(path: &str, max: usize) -> String {
    if path.chars().count() <= max {
        return path.to_string();
    }
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let mut tail = String::new();
    for part in parts.iter().rev() {
        let next = if tail.is_empty() { part.to_string() } else { format!("{part}/{tail}") };
        // "…/" takes two characters.
        if next.chars().count() + 2 > max && !tail.is_empty() {
            break;
        }
        tail = next;
    }
    format!("…/{tail}")
}

/// What a row's second line says when the terminal has nothing to report (an idle agent, a bare
/// shell), so no row is a single line: where it is, `~/…` and cut from the front to `max`
/// characters. `None` only when the terminal's folder is unknown.
pub fn place_detail(cwd: Option<&Path>, home: Option<&Path>, max: usize) -> Option<String> {
    Some(path_tail(&display_path(cwd?, home), max))
}

/// `~/Projects/app` for paths under the home directory, the full path otherwise.
pub fn display_path(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{agent_detail, ago, clock, display_path, path_tail, place_detail, took};
    use crate::view::sidebar::RowState;
    use std::path::Path;
    use std::time::Duration;

    const fn mins(n: u64) -> Duration {
        Duration::from_secs(n * 60)
    }

    #[test]
    fn an_agents_second_line_says_what_it_does() {
        let ask = Some("Approve cargo test?");
        assert_eq!(
            agent_detail(RowState::NeedsYou, ask, mins(1)).as_deref(),
            Some("Needs input · Approve cargo test?")
        );
        assert_eq!(agent_detail(RowState::NeedsYou, Some("  "), mins(1)).as_deref(), Some("Needs input"));
        assert_eq!(
            agent_detail(RowState::Working, Some("Editing sidebar.rs…"), mins(3)).as_deref(),
            Some("Editing sidebar.rs…")
        );
        assert_eq!(agent_detail(RowState::Working, None, mins(3)).as_deref(), Some("Working · 3m"));
        // Minutes only: the first minute reads "<1m" and doesn't tick.
        assert_eq!(
            agent_detail(RowState::Working, None, Duration::from_secs(20)).as_deref(),
            Some("Working · <1m")
        );
        assert_eq!(agent_detail(RowState::Finished, None, mins(12)).as_deref(), Some("Finished · 12m ago"));
        assert_eq!(agent_detail(RowState::Finished, None, mins(0)).as_deref(), Some("Finished · just now"));
        assert_eq!(agent_detail(RowState::Idle, ask, mins(5)), None, "an idle agent is one line");
        // A result says its own words when it has them, else when it happened.
        assert_eq!(
            agent_detail(RowState::Finished, Some("Upgraded 12 packages"), mins(2)).as_deref(),
            Some("Finished · Upgraded 12 packages")
        );
        assert_eq!(agent_detail(RowState::Failed, None, mins(2)).as_deref(), Some("Failed · 2m ago"));
        assert_eq!(
            agent_detail(RowState::Failed, Some("rsync failed"), mins(2)).as_deref(),
            Some("Failed · rsync failed")
        );
        assert_eq!(agent_detail(RowState::Failed, Some(" "), mins(0)).as_deref(), Some("Failed · just now"));
    }

    #[test]
    fn times_read_short() {
        assert_eq!(took(Duration::from_secs(59)), None);
        assert_eq!(took(mins(3)).as_deref(), Some("3m"));
        assert_eq!(took(mins(61)).as_deref(), Some("1h"));
        assert_eq!(took(mins(60 * 49)).as_deref(), Some("2d"));
        assert_eq!(ago(mins(60)), "1h ago");
        assert_eq!(clock(Duration::from_secs(5)), "0:05");
        assert_eq!(clock(Duration::from_secs(72)), "1:12");
        assert_eq!(clock(Duration::from_secs(3723)), "1:02:03");
    }

    #[test]
    fn paths_give_way_at_the_front() {
        assert_eq!(path_tail("~/Projects/app", 20), "~/Projects/app");
        assert_eq!(path_tail("~/Projects/clients/acme/app", 18), "…/clients/acme/app");
        assert_eq!(
            path_tail("/a/very-long-folder-name", 6),
            "…/very-long-folder-name",
            "one component stays whole"
        );
    }

    #[test]
    fn a_quiet_row_says_where_it_is() {
        let home = Some(Path::new("/Users/me"));
        let cwd = Path::new("/Users/me/Projects/app");
        assert_eq!(place_detail(Some(cwd), home, 38).as_deref(), Some("~/Projects/app"));
        assert_eq!(place_detail(Some(Path::new("/opt/x")), home, 38).as_deref(), Some("/opt/x"));
        // A long path is cut from the front.
        let long = Path::new("/Users/me/Projects/clients/acme/very/deep/app");
        assert_eq!(place_detail(Some(long), home, 20).as_deref(), Some("…/acme/very/deep/app"));
        // Without a folder there is nothing to say.
        assert_eq!(place_detail(None, home, 38), None);
    }

    #[test]
    fn home_is_abbreviated() {
        let home = Path::new("/Users/me");
        assert_eq!(display_path(Path::new("/Users/me/Projects/x"), Some(home)), "~/Projects/x");
        assert_eq!(display_path(Path::new("/opt/x"), Some(home)), "/opt/x");
        assert_eq!(display_path(Path::new("/Users/me"), Some(home)), "~");
    }
}

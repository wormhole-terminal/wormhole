//! `Wormhole sessions list | end <id> | end --all | forget <id>`: the session hosts that keep
//! terminals running behind the app (`repo-docs/docs/session-host.md`), seen and ended from the
//! command line. Unlike the rest of the control protocol none of this goes through the app: it
//! works from the files in `<data dir>/hosts/` and talks to each host's own socket, so it works
//! with the app closed, which is exactly when someone wants it. The same scan makes `list` and
//! `status` useful when the app is not running (`offline`).

use std::path::Path;
use std::time::Duration;

use serde::Serialize;

use super::Reply;
use super::cli::{print, print_err};
use crate::ipc::hosts::{self, ForgetError, HostMeta, Scan, StaleReason};

/// How long one host gets to take `Kill` and hang up: its shell's hangup grace (3 s) and a margin.
/// Past it the host is taken to be stuck, and the signal takes over.
const END_WAIT: Duration = Duration::from_secs(5);

/// What `sessions` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    List { json: bool },
    End(String),
    EndAll,
    Forget(String),
}

/// The usage text of the `sessions` verb.
pub fn help(wh: &str) -> String {
    format!(
        "\
  {wh} sessions list [--json]
      The terminals that keep running behind the app: id, pid, shell pid, start, version, live or stale.
      Works while the app is closed.
  {wh} sessions end <id> | --all
      End one background terminal (its shell gets a hangup), or every one of this data folder.
  {wh} sessions forget <id>
      Remove the files of a stale session (its process is gone). A live one is refused.
"
    )
}

/// Read the words after `sessions`.
pub fn parse(args: &[String]) -> Result<Command, String> {
    let (verb, rest) = args.split_first().ok_or("sessions needs list, end or forget")?;
    let mut json = false;
    let mut all = false;
    let mut ids: Vec<&str> = Vec::new();
    for arg in rest {
        match arg.as_str() {
            "--json" if verb == "list" => json = true,
            "--all" if verb == "end" => all = true,
            flag if flag.starts_with('-') => {
                return Err(format!("unknown option {flag} for sessions {verb}"));
            }
            id => ids.push(id),
        }
    }
    let one_id = |what: &str| -> Result<String, String> {
        let [id] = ids.as_slice() else { return Err(format!("sessions {what} takes one session id")) };
        if !hosts::is_valid_host_id(id) {
            return Err(format!("{id:?} is not a session id (12 lowercase letters and digits)"));
        }
        Ok((*id).to_string())
    };
    match verb.as_str() {
        "list" if ids.is_empty() => Ok(Command::List { json }),
        "list" => Err("sessions list takes no id".into()),
        "end" if all && ids.is_empty() => Ok(Command::EndAll),
        "end" if all => Err("sessions end takes an id or --all, not both".into()),
        "end" => one_id("end <id> (or end --all)").map(Command::End),
        "forget" => one_id("forget").map(Command::Forget),
        other => Err(format!("unknown sessions command {other:?} (list, end, forget)")),
    }
}

/// `Wormhole sessions …`: returns the exit code (0 done, 1 something would not end or be forgotten,
/// 2 a mistake in the arguments).
pub fn run(args: &[String]) -> i32 {
    let command = match parse(args) {
        Ok(command) => command,
        Err(mistake) => {
            print_err(&format!("Wormhole: {mistake}\n\n{}", help(super::COMMAND_NAME)));
            return 2;
        }
    };
    let hosts_dir = hosts::hosts_dir(&crate::storage::paths::data_dir());
    let (code, out, err) = execute(&command, &hosts_dir, END_WAIT);
    print(&out);
    print_err(&err);
    code
}

/// `sessions <args>` against `hosts_dir` instead of this data folder's, for devctl and the tests:
/// the exit code, what goes to stdout and what goes to stderr.
pub fn run_in(args: &[String], hosts_dir: &Path) -> (i32, String, String) {
    match parse(args) {
        Ok(command) => execute(&command, hosts_dir, END_WAIT),
        Err(mistake) => (2, String::new(), format!("Wormhole: {mistake}\n")),
    }
}

/// Do `command` on the hosts in `hosts_dir`: the exit code, the text for stdout and for stderr.
fn execute(command: &Command, hosts_dir: &Path, wait: Duration) -> (i32, String, String) {
    let scanned = match hosts::scan(hosts_dir) {
        Ok(scanned) => scanned,
        Err(err) => return (1, String::new(), format!("Wormhole: reading {}: {err}\n", hosts_dir.display())),
    };
    match command {
        Command::List { json } => {
            let text = if *json {
                format!("{}\n", serde_json::to_string(&Rows::of(&scanned)).unwrap_or_default())
            } else {
                list_text(&scanned)
            };
            (0, text, String::new())
        }
        Command::End(id) => end_one(&scanned, hosts_dir, id, wait),
        Command::EndAll => end_everything(&scanned, wait),
        Command::Forget(id) => match hosts::forget(hosts_dir, id) {
            Ok(done) => (0, format!("forgot {id} ({} files removed)\n", done.removed.len()), String::new()),
            Err(ForgetError::Unknown) => (1, String::new(), format!("Wormhole: no session {id}\n")),
            Err(err) => (1, String::new(), format!("Wormhole: {id}: {err}\n")),
        },
    }
}

fn end_one(scanned: &Scan, hosts_dir: &Path, id: &str, wait: Duration) -> (i32, String, String) {
    let fail = |why: String| (1, String::new(), format!("Wormhole: {why}\n"));
    if let Some(stale) = scanned.stale.iter().find(|s| s.id == id) {
        // The process runs but its socket is gone: it can still be signalled. Anything else is dead.
        if stale.reason != StaleReason::SocketMissing {
            return fail(format!(
                "session {id} is not running ({}); `sessions forget {id}` removes its files",
                reason_word(&stale.reason)
            ));
        }
    }
    let meta = match hosts::find(hosts_dir, id) {
        Ok(meta) => meta,
        Err(why) => return fail(why),
    };
    match hosts::end_host(&meta, wait) {
        Ok(how) => (0, format!("{id} {}\n", how.word()), String::new()),
        Err(why) => fail(format!("session {id} did not end: {why}")),
    }
}

fn end_everything(scanned: &Scan, wait: Duration) -> (i32, String, String) {
    let (mut out, mut err, mut failed) = (String::new(), String::new(), 0);
    for (id, result) in hosts::end_all(&scanned.live, wait) {
        match result {
            Ok(how) => out.push_str(&format!("{id} {}\n", how.word())),
            Err(why) => {
                failed += 1;
                err.push_str(&format!("Wormhole: session {id} did not end: {why}\n"));
            }
        }
    }
    if scanned.live.is_empty() {
        out.push_str("no sessions running\n");
    }
    if !scanned.stale.is_empty() {
        out.push_str(&format!("{} stale: `sessions forget <id>` removes their files\n", scanned.stale.len()));
    }
    (i32::from(failed > 0), out, err)
}

/// Every host, live first, as the table `sessions list` prints.
fn list_text(scanned: &Scan) -> String {
    let rows = Rows::of(scanned).hosts;
    if rows.is_empty() {
        return "no sessions\n".into();
    }
    let mut out = format!(
        "{:<12}  {:>7}  {:>7}  {:<20}  {:<8}  {}\n",
        "ID", "PID", "CHILD", "STARTED", "VERSION", "STATE"
    );
    for row in rows {
        let started = row.started_at.map_or("-".to_string(), utc);
        out.push_str(&format!(
            "{:<12}  {:>7}  {:>7}  {:<20}  {:<8}  {}\n",
            row.id,
            row.pid.map_or("-".to_string(), |p| p.to_string()),
            row.child_pid.map_or("-".to_string(), |p| p.to_string()),
            started,
            row.host_version.as_deref().unwrap_or("-"),
            row.state_text(),
        ));
    }
    out
}

/// One host as `list` shows and `--json` prints it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Row {
    id: String,
    /// `live` or `stale`.
    state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    child_pid: Option<u32>,
    /// Unix seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    host_version: Option<String>,
}

impl Row {
    fn live(meta: &HostMeta) -> Self {
        Self::of(meta, "live", None)
    }

    fn of(meta: &HostMeta, state: &'static str, reason: Option<String>) -> Self {
        Self {
            id: meta.id.clone(),
            state,
            reason,
            pid: Some(meta.pid),
            child_pid: meta.child_pid,
            started_at: Some(meta.started_at),
            host_version: Some(meta.host_version.clone()),
        }
    }

    fn state_text(&self) -> String {
        match &self.reason {
            Some(reason) => format!("{} ({reason})", self.state),
            None => self.state.to_string(),
        }
    }
}

/// All hosts of a scan: the live ones, oldest first, then the stale ones by id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Rows {
    hosts: Vec<Row>,
}

impl Rows {
    fn of(scanned: &Scan) -> Self {
        let live = scanned.live.iter().map(Row::live);
        let stale = scanned.stale.iter().map(|stale| {
            let reason = Some(reason_word(&stale.reason));
            match &stale.meta {
                Some(meta) => Row::of(meta, "stale", reason),
                None => Row {
                    id: stale.id.clone(),
                    state: "stale",
                    reason,
                    pid: None,
                    child_pid: None,
                    started_at: None,
                    host_version: None,
                },
            }
        });
        Self { hosts: live.chain(stale).collect() }
    }
}

fn reason_word(reason: &StaleReason) -> String {
    match reason {
        StaleReason::Unreadable(_) => "unreadable meta",
        StaleReason::PidGone => "process gone",
        StaleReason::SocketMissing => "socket missing",
        StaleReason::NoMeta => "no meta",
    }
    .to_string()
}

/// Unix seconds as `2025-10-09 08:53:20Z`. UTC: the table is read by people and by bug reports,
/// and a time zone database is not worth a dependency here.
fn utc(secs: u64) -> String {
    let (days, rest) = (secs / 86_400, secs % 86_400);
    // Howard Hinnant's days-to-civil, for days since 1970-01-01.
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}Z", rest / 3600, rest % 3600 / 60, rest % 60)
}

/// What `list` and `status` say when the app is not running but terminals are: the app is not
/// there to ask, so the hosts' own files answer. `None` when there is nothing to add (no live host,
/// or a question only the app can answer), and the caller says "Wormhole isn't running" as before.
pub(super) fn offline(action: &super::Action, json: bool) -> Option<String> {
    let hosts_dir = hosts::hosts_dir(&crate::storage::paths::data_dir());
    offline_in(action, json, &hosts_dir)
}

fn offline_in(action: &super::Action, json: bool, hosts_dir: &Path) -> Option<String> {
    use super::Action;
    let live = hosts::scan(hosts_dir).ok()?.live;
    if live.is_empty() || !matches!(action, Action::List | Action::Status { target: None }) {
        return None;
    }
    if json {
        #[derive(Serialize)]
        struct Offline {
            app_running: bool,
            hosts: Vec<Row>,
        }
        let hosts = live.iter().map(Row::live).collect();
        let reply = Reply::ok(Offline { app_running: false, hosts });
        return Some(format!("{}\n", serde_json::to_string(&reply).unwrap_or_default()));
    }
    let wh = super::COMMAND_NAME;
    Some(match action {
        Action::List => {
            let scanned = Scan { live, stale: Vec::new() };
            format!("app not running\n{}", list_text(&scanned))
        }
        _ => format!(
            "app not running: {} still {} in the background (`{wh} sessions list` shows them)\n",
            if live.len() == 1 { "1 terminal".to_string() } else { format!("{} terminals", live.len()) },
            if live.len() == 1 { "runs" } else { "run" }
        ),
    })
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;

    use super::*;
    use crate::ipc::control::Action;

    fn words(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_sessions_verbs_parse() {
        assert_eq!(parse(&words(&["list"])), Ok(Command::List { json: false }));
        assert_eq!(parse(&words(&["list", "--json"])), Ok(Command::List { json: true }));
        assert_eq!(parse(&words(&["end", "--all"])), Ok(Command::EndAll));
        assert_eq!(parse(&words(&["end", "abcdefghij12"])), Ok(Command::End("abcdefghij12".into())));
        assert_eq!(parse(&words(&["forget", "abcdefghij12"])), Ok(Command::Forget("abcdefghij12".into())));
    }

    #[test]
    fn mistakes_in_the_arguments_are_named() {
        let mistake = |list: &[&str]| parse(&words(list)).expect_err("refused");
        assert!(mistake(&[]).contains("needs"));
        assert!(mistake(&["frobnicate"]).contains("unknown sessions command"));
        assert!(mistake(&["end"]).contains("takes one session id"));
        assert!(mistake(&["end", "../etc"]).contains("not a session id"));
        assert!(mistake(&["end", "ABCDEFGHIJ12"]).contains("not a session id"));
        assert!(mistake(&["end", "--all", "abcdefghij12"]).contains("not both"));
        assert!(mistake(&["forget"]).contains("takes one session id"));
        assert!(mistake(&["forget", "--all"]).contains("unknown option"));
        assert!(mistake(&["list", "abcdefghij12"]).contains("no id"));
        assert!(mistake(&["list", "--all"]).contains("unknown option"));
    }

    #[test]
    fn times_read_as_utc() {
        assert_eq!(utc(0), "1970-01-01 00:00:00Z");
        assert_eq!(utc(1_760_000_000), "2025-10-09 08:53:20Z");
        assert_eq!(utc(951_782_400), "2000-02-29 00:00:00Z", "a leap day");
        assert_eq!(utc(4_102_444_799), "2099-12-31 23:59:59Z");
    }

    fn short_dir() -> tempfile::TempDir {
        tempfile::Builder::new().prefix("hc").tempdir_in("/tmp").expect("temp")
    }

    fn write_meta(dir: &Path, id: &str, pid: u32, started_at: u64) -> std::path::PathBuf {
        let socket = dir.join(format!("{id}.sock"));
        let text = format!(
            "format=1\nid={id}\npid={pid}\nchild_pid=4242\nstarted_at={started_at}\nhost_version=0.3.0\n\
             proto_min=1\nproto_max=1\nsocket={}\ndata_dir=/d\n",
            socket.display()
        );
        std::fs::write(dir.join(format!("{id}.meta")), text).expect("meta");
        socket
    }

    fn dead_pid() -> u32 {
        let mut child = std::process::Command::new("/usr/bin/true").spawn().expect("spawns");
        let pid = child.id();
        child.wait().expect("waits");
        pid
    }

    /// A live host (this process's pid, a bound socket) and a stale one (its pid is gone).
    fn two_hosts(dir: &Path) -> UnixListener {
        let socket = write_meta(dir, "aaaaaaaaaaaa", std::process::id(), 1_760_000_000);
        let listener = UnixListener::bind(&socket).expect("binds");
        write_meta(dir, "bbbbbbbbbbbb", dead_pid(), 1_700_000_000);
        listener
    }

    #[test]
    fn list_shows_live_hosts_first_then_the_stale_with_their_reason() {
        let dir = short_dir();
        let _live = two_hosts(dir.path());
        let (code, out, err) = execute(&Command::List { json: false }, dir.path(), END_WAIT);
        assert_eq!((code, err.as_str()), (0, ""));
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3, "{out}");
        assert!(lines[0].starts_with("ID") && lines[0].ends_with("STATE"));
        assert!(lines[1].starts_with("aaaaaaaaaaaa") && lines[1].contains("2025-10-09 08:53:20Z"), "{out}");
        assert!(lines[1].contains("4242") && lines[1].contains("0.3.0") && lines[1].ends_with("live"));
        assert!(lines[2].starts_with("bbbbbbbbbbbb") && lines[2].ends_with("stale (process gone)"), "{out}");
    }

    #[test]
    fn list_as_json_is_one_object_and_an_empty_folder_says_so() {
        let dir = short_dir();
        let (_, out, _) = execute(&Command::List { json: false }, dir.path(), END_WAIT);
        assert_eq!(out, "no sessions\n");
        let _live = two_hosts(dir.path());
        let (_, out, _) = execute(&Command::List { json: true }, dir.path(), END_WAIT);
        let value: serde_json::Value = serde_json::from_str(out.trim()).expect("json");
        assert_eq!(value["hosts"][0]["id"], "aaaaaaaaaaaa");
        assert_eq!(value["hosts"][0]["state"], "live");
        assert_eq!(value["hosts"][1]["state"], "stale");
        assert_eq!(value["hosts"][1]["reason"], "process gone");
    }

    #[test]
    fn forget_removes_a_stale_host_and_refuses_a_live_one() {
        let dir = short_dir();
        let _live = two_hosts(dir.path());
        let (code, out, _) = execute(&Command::Forget("bbbbbbbbbbbb".into()), dir.path(), END_WAIT);
        assert_eq!(code, 0, "{out}");
        assert!(!dir.path().join("bbbbbbbbbbbb.meta").exists());
        let (code, _, err) = execute(&Command::Forget("aaaaaaaaaaaa".into()), dir.path(), END_WAIT);
        assert_eq!(code, 1);
        assert!(err.contains("end it instead"), "{err}");
        assert!(dir.path().join("aaaaaaaaaaaa.meta").exists());
        let (code, _, err) = execute(&Command::Forget("cccccccccccc".into()), dir.path(), END_WAIT);
        assert_eq!((code, err.as_str()), (1, "Wormhole: no session cccccccccccc\n"));
    }

    #[test]
    fn ending_a_stale_host_points_at_forget_and_an_unknown_one_is_named() {
        let dir = short_dir();
        let _live = two_hosts(dir.path());
        let (code, _, err) = execute(&Command::End("bbbbbbbbbbbb".into()), dir.path(), END_WAIT);
        assert_eq!(code, 1);
        assert!(err.contains("not running") && err.contains("sessions forget bbbbbbbbbbbb"), "{err}");
        let (code, _, err) = execute(&Command::End("cccccccccccc".into()), dir.path(), END_WAIT);
        assert_eq!((code, err.as_str()), (1, "Wormhole: no session cccccccccccc\n"));
    }

    #[test]
    fn end_all_with_nothing_running_says_so_and_counts_the_stale() {
        let dir = short_dir();
        let (code, out, _) = execute(&Command::EndAll, dir.path(), END_WAIT);
        assert_eq!((code, out.as_str()), (0, "no sessions running\n"));
        write_meta(dir.path(), "bbbbbbbbbbbb", dead_pid(), 1);
        let (code, out, _) = execute(&Command::EndAll, dir.path(), END_WAIT);
        assert_eq!(code, 0);
        assert!(out.contains("1 stale"), "{out}");
    }

    #[test]
    fn a_missing_hosts_folder_is_an_empty_list() {
        let dir = short_dir();
        let (code, out, _) = execute(&Command::List { json: false }, &dir.path().join("none"), END_WAIT);
        assert_eq!((code, out.as_str()), (0, "no sessions\n"));
    }

    #[test]
    fn list_and_status_add_the_live_hosts_when_the_app_is_not_running() {
        let dir = short_dir();
        assert_eq!(offline_in(&Action::List, false, dir.path()), None, "nothing to add");
        let _live = two_hosts(dir.path());
        let text = offline_in(&Action::List, false, dir.path()).expect("lists");
        assert!(text.starts_with("app not running\nID"), "{text}");
        assert!(text.contains("aaaaaaaaaaaa") && !text.contains("bbbbbbbbbbbb"), "live ones only: {text}");
        let status = offline_in(&Action::Status { target: None }, false, dir.path()).expect("status");
        assert!(status.starts_with("app not running: 1 terminal still runs"), "{status}");
        let json = offline_in(&Action::List, true, dir.path()).expect("json");
        let value: serde_json::Value = serde_json::from_str(json.trim()).expect("json");
        assert_eq!(value["result"]["app_running"], false);
        assert_eq!(value["result"]["hosts"][0]["id"], "aaaaaaaaaaaa");
        let target = Action::Status { target: Some(crate::ipc::control::Target::Tab(3)) };
        assert_eq!(offline_in(&target, false, dir.path()), None, "only the app knows its tabs");
        assert_eq!(
            offline_in(
                &Action::Notify { target: None, title: String::new(), body: "x".into() },
                false,
                dir.path()
            ),
            None
        );
    }
}

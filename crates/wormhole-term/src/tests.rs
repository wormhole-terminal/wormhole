//! Real PTYs running small programs (`/bin/cat`, `/bin/sh -c …`), never the
//! user's shell. Nothing here polls the event receiver unless a test says so,
//! which is how a UI thread busy elsewhere looks to the terminal.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use wormhole_vt::types::Line;

use super::{
    BlockState, Direction, GridSize, Mark, Notification, Progress, ProgressState, PromptKind, Pty, PtyEvent,
    StartOptions,
};

const SIZE: GridSize = GridSize { cols: 80, rows: 24, cell_width: 8.0, cell_height: 16.0 };

pub(crate) fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub(crate) fn spawn(program: &str, args: &[&str]) -> (Pty, super::PtyEvents) {
    Pty::spawn_program(SIZE, &StartOptions::default(), program, args).expect("spawn")
}

#[test]
fn output_wakes_the_ui_once_until_it_looks() {
    let (pty, events) = spawn("/bin/cat", &[]);
    for i in 0..200 {
        pty.write(format!("{i}\n").into_bytes());
        if i % 10 == 0 {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    wait_until("the last echo", || pty.screen_text().contains("199"));
    let revision = pty.output_revision();
    assert!(revision > 1, "several bursts were parsed (revision {revision})");
    assert_eq!(events.queued(), 1, "one Output stands for all of them");
    assert!(matches!(events.try_recv(), Some(PtyEvent::Output)));
    assert_eq!(events.queued(), 0);

    // Taking it re-arms the next one.
    pty.write(b"again\n".to_vec());
    wait_until("the next Output", || events.queued() == 1);
    assert!(pty.output_revision() > revision);
    assert!(matches!(events.try_recv(), Some(PtyEvent::Output)));
}

#[test]
fn terminal_queries_are_answered_without_the_ui() {
    // The program asks for the cursor position and prints the reply with ESC shown as `E`
    // (`od`'s spacing differs between GNU and BSD, `tr` does not); nobody reads `events`,
    // so the reply must come from the PTY thread.
    let script = r#"stty raw -echo; printf '\033[6n'; r=$(dd bs=1 count=6 2>/dev/null); stty sane; printf '%s' "$r" | tr '\033' E; sleep 5"#;
    let (pty, _events) = spawn("/bin/sh", &["-c", script]);
    wait_until("the reply", || pty.screen_text().contains("E[1;1R"));
}

#[test]
fn a_reply_the_ui_gives_comes_before_a_cursor_reply_asked_after_it() {
    // A program reads the clipboard (OSC 52, which the UI answers) and then asks for the cursor
    // position; the cursor reply must not overtake the clipboard's, or the program takes it for
    // the answer to the first and the clipboard reply, landing after, is typed at the prompt. The
    // reply is `ESC ] 52 ; c ; aGk= ESC \` (13 bytes) and `ESC [ 1 ; 1 R` (6).
    let script = r#"stty raw -echo; printf '\033]52;c;?\033\\\033[6n'; r=$(dd bs=1 count=19 2>/dev/null); stty sane; printf '%s' "$r" | tr '\033' E; sleep 5"#;
    let (pty, events) = spawn("/bin/sh", &["-c", script]);
    let deadline = Instant::now() + Duration::from_secs(10);
    let (format, reply) = loop {
        assert!(Instant::now() < deadline, "timed out waiting for the clipboard request");
        match events.try_recv() {
            Some(PtyEvent::ClipboardLoad(format, reply)) => break (format, reply),
            Some(_) => {}
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    // A busy UI: the cursor reply is long done on the PTY thread by now.
    std::thread::sleep(Duration::from_millis(200));
    reply.send(format("hi").into_bytes());
    wait_until("both replies", || pty.screen_text().contains(r"E]52;c;aGk=E\E[1;1R"));
}

#[test]
fn a_cursor_reply_asked_before_a_clipboard_read_in_the_same_read_comes_first() {
    // Both sequences arrive in one read of the master. The cursor reply is `ESC [ 1 ; 1 R` (6
    // bytes), the clipboard's `ESC ] 52 ; c ; aGk= ESC \` (13).
    let script = r#"stty raw -echo; printf '\033[6n\033]52;c;?\033\\'; r=$(dd bs=1 count=19 2>/dev/null); stty sane; printf '%s' "$r" | tr '\033' E; sleep 5"#;
    let (pty, events) = spawn("/bin/sh", &["-c", script]);
    let deadline = Instant::now() + Duration::from_secs(10);
    let (format, reply) = loop {
        assert!(Instant::now() < deadline, "timed out waiting for the clipboard request");
        match events.try_recv() {
            Some(PtyEvent::ClipboardLoad(format, reply)) => break (format, reply),
            Some(_) => {}
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    std::thread::sleep(Duration::from_millis(200));
    reply.send(format("hi").into_bytes());
    wait_until("both replies", || pty.screen_text().contains(r"E[1;1RE]52;c;aGk=E\"));
}

#[test]
fn a_query_the_ui_drops_lets_the_replies_behind_it_go() {
    let script = r#"stty raw -echo; printf '\033]52;c;?\033\\\033[6n'; r=$(dd bs=1 count=6 2>/dev/null); stty sane; printf '%s' "$r" | tr '\033' E; sleep 5"#;
    let (pty, events) = spawn("/bin/sh", &["-c", script]);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < deadline, "timed out waiting for the clipboard request");
        match events.try_recv() {
            // Dropped unanswered, as when the view is gone.
            Some(PtyEvent::ClipboardLoad(..)) => break,
            Some(_) => {}
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    wait_until("the cursor reply", || pty.screen_text().contains("E[1;1R"));
}

#[test]
fn text_area_size_queries_are_answered_without_the_ui() {
    // CSI 14 t: the text area in pixels, `CSI 4 ; height ; width t`.
    let script = r#"stty raw -echo; printf '\033[14t'; r=$(dd bs=1 count=12 2>/dev/null); stty sane; printf '%s' "$r" | tr '\033' E; sleep 5"#;
    let (pty, _events) = spawn("/bin/sh", &["-c", script]);
    wait_until("the size reply", || pty.screen_text().contains("E[4;384;640t"));
}

#[test]
fn exit_is_reported_once() {
    let (_pty, events) = spawn("/bin/sh", &["-c", "printf done"]);
    let mut exited = 0;
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        match events.try_recv() {
            Some(PtyEvent::Exited(_)) => exited += 1,
            Some(_) => {}
            None if exited > 0 => break,
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    // Let a late duplicate (ChildExit and Exit both arrive) show up.
    std::thread::sleep(Duration::from_millis(100));
    while let Some(event) = events.try_recv() {
        exited += usize::from(matches!(event, PtyEvent::Exited(_)));
    }
    assert_eq!(exited, 1);
}

#[test]
fn the_scrollback_source_works_from_another_thread() {
    let (pty, _events) = spawn("/bin/sh", &["-c", "printf 'one\\ntwo\\n'; sleep 5"]);
    wait_until("the output", || pty.screen_text().contains("two"));
    let source = pty.scrollback();
    let text = std::thread::spawn(move || source.text(100)).join().expect("thread");
    assert_eq!(text.as_deref(), Some("one\ntwo"));
}

/// UI wakeups for streaming output: the engine's wakeups (each one an event in
/// the queue before coalescing) against the events delivered now, with a UI
/// that looks every 16 ms. Run with `--ignored --nocapture`.
#[test]
#[ignore = "measurement; run with --ignored --nocapture"]
fn streaming_output_wakes_the_ui_once_per_look() {
    let script = "i=0; while [ $i -lt 20000 ]; do echo \"line $i of streamed agent output\"; i=$((i+1)); done; sleep 5";
    let (pty, events) = spawn("/bin/sh", &["-c", script]);
    let start = Instant::now();
    let mut delivered = 0;
    let mut most_queued = 0;
    while !pty.screen_text().contains("line 19999") {
        assert!(start.elapsed() < Duration::from_secs(30), "the program never finished");
        std::thread::sleep(Duration::from_millis(16));
        most_queued = most_queued.max(events.queued());
        while events.try_recv().is_some() {
            delivered += 1;
        }
    }
    let wakeups = pty.output_revision();
    eprintln!(
        "{wakeups} wakeups (queued events before), {delivered} delivered now, at most {most_queued} queued, in {:?}",
        start.elapsed()
    );
    assert!(delivered as u64 <= wakeups && most_queued <= 2);
}

#[test]
fn an_exited_shell_keeps_its_screen_and_forgets_its_process() {
    // A project command that fails at once: its message must stay readable.
    let (pty, events) = spawn("/bin/sh", &["-c", "echo 'zsh: command not found: claud'"]);
    wait_until("the exit", || matches!(events.try_recv(), Some(PtyEvent::Exited(_))));
    assert!(!pty.is_alive());
    assert!(pty.screen_text().contains("command not found"));
    assert_eq!(pty.scrollback().text(100).as_deref(), Some("zsh: command not found: claud"));
    // Its pid may already belong to another process.
    assert_eq!(pty.cwd(), None);
    assert_eq!(pty.program_in_foreground(), None);
}

/// Everything `events` has queued right now, in order.
pub(crate) fn drain(events: &super::PtyEvents) -> Vec<PtyEvent> {
    std::iter::from_fn(|| events.try_recv()).collect()
}

#[test]
fn the_shells_reported_folder_wins_while_its_process_stays_put() {
    let here = tempfile::tempdir().expect("temp dir");
    let reported = here.path().to_path_buf();
    // Reported: an existing local folder (BEL), then one on another host and one that isn't
    // here (both ignored). Then, after the UI (this test) has looked, the shell moves without
    // saying so.
    let script = format!(
        "printf '\\033]7;file://localhost{}\\a'; printf 'reported\\n'; \
         printf '\\033]7;file://build.example.com/etc\\033\\\\'; printf '\\033]7;file:///no/such/folder\\a'; \
         printf 'ignored\\n'; sleep 1; cd /; printf 'moved\\n'; sleep 5",
        reported.display()
    );
    let (pty, events) = spawn("/bin/sh", &["-c", &script]);
    wait_until("the report", || pty.screen_text().contains("reported"));
    wait_until("the folder", || pty.cwd() == Some(reported.clone()));
    assert!(drain(&events).iter().any(|e| matches!(e, PtyEvent::Cwd)));
    wait_until("the ignored reports", || pty.screen_text().contains("ignored"));
    assert_eq!(pty.cwd(), Some(reported.clone()));
    assert!(!drain(&events).iter().any(|e| matches!(e, PtyEvent::Cwd)), "nothing to report");
    // Polling sees the move: a shell without integration is in charge now.
    wait_until("the move", || pty.screen_text().contains("moved"));
    wait_until("polling to win", || pty.cwd() == Some(PathBuf::from("/")));
}

#[test]
fn prompt_marks_give_the_exit_code_the_prompt_state_and_the_prompt_lines() {
    // Two prompts with a command between them, as a shell with integration prints them.
    let prompt = "\\033]133;A\\a$ \\033]133;B\\a";
    let script = format!(
        "printf '{prompt}'; printf 'ls\\n'; printf '\\033]133;C\\a'; printf 'out\\n'; \
         printf '\\033]133;D;3\\a'; printf '{prompt}'; printf 'ready'; sleep 5"
    );
    let (pty, events) = spawn("/bin/sh", &["-c", &script]);
    wait_until("the second prompt", || pty.screen_text().contains("$ ready"));
    let marks: Vec<Mark> = drain(&events)
        .into_iter()
        .filter_map(|e| match e {
            PtyEvent::Mark(mark) => Some(mark),
            _ => None,
        })
        .collect();
    assert_eq!(
        marks,
        [
            Mark::PromptStart(PromptKind::Initial),
            Mark::CommandStart,
            Mark::OutputStart,
            Mark::Finished(Some(3)),
            Mark::PromptStart(PromptKind::Initial),
            Mark::CommandStart,
        ]
    );
    assert_eq!(pty.last_exit_code(), Some(3));
    assert_eq!(pty.at_prompt(), Some(true));
    assert_eq!(pty.screen_text().lines().take(3).collect::<Vec<_>>(), ["$ ls", "out", "$ ready"]);
    assert_eq!(pty.prompt_lines(), [Line(0), Line(2)]);
    assert_eq!(pty.prompt_line(Line(2), Direction::Up), Some(Line(0)));
    assert_eq!(pty.prompt_line(Line(0), Direction::Up), None);
    assert_eq!(pty.prompt_line(Line(0), Direction::Down), Some(Line(2)));
    assert_eq!(pty.prompt_line(Line(2), Direction::Down), None);
    // The same as blocks: the command, its output and how it went, timed on the PTY thread.
    assert!(pty.has_blocks());
    let blocks = pty.blocks();
    assert_eq!(blocks.len(), 2);
    assert_eq!((blocks[0].command.as_str(), blocks[0].output_lines()), ("ls", Some((Line(1), Line(1)))));
    assert!(matches!(blocks[0].state, BlockState::Finished { code: Some(3), took: Some(_) }));
    assert_eq!(pty.lines_text(Line(1), Line(1)), "out");
    assert_eq!(blocks[1].state, BlockState::Prompt);
    assert_eq!(pty.block_at(Line(1)).map(|b| b.id), Some(1));
    assert_eq!(pty.block(2).map(|b| b.start), Some(Line(2)));
    let mut snapshot = super::ScreenSnapshot::default();
    pty.snapshot(&mut snapshot, 0);
    let spans: Vec<(u64, bool)> = snapshot.block_spans().iter().map(|s| (s.id, s.state.failed())).collect();
    assert_eq!(spans, [(1, true), (2, false)]);
}

#[test]
fn prompt_marks_follow_their_lines_into_the_history() {
    let prompt = "\\033]133;A\\a$ \\033]133;B\\a";
    let script = format!(
        "printf '{prompt}'; printf 'seq\\n\\033]133;C\\a'; i=0; while [ $i -lt 40 ]; do echo \"line $i\"; i=$((i+1)); done; \
         printf '\\033]133;D;0\\a{prompt}'; printf 'ready'; sleep 5"
    );
    let options = StartOptions { scrollback_lines: 1000, ..StartOptions::default() };
    let (pty, _events) = Pty::spawn_program(SIZE, &options, "/bin/sh", &["-c", &script]).expect("spawn");
    wait_until("the second prompt", || pty.screen_text().contains("$ ready"));
    let mut snapshot = super::ScreenSnapshot::default();
    pty.snapshot(&mut snapshot, 0);
    let history = snapshot.history_size as i32;
    assert!(history > 0, "the first prompt scrolled off the screen");
    // The first prompt is at the top of everything, the second on the screen's last used row.
    assert_eq!(pty.prompt_lines(), [Line(-history), Line(SIZE.rows as i32 - 1)]);
    assert_eq!(pty.prompt_line(Line(0), Direction::Up), Some(Line(-history)));
}

#[test]
fn notifications_and_progress_are_reported() {
    let script = "printf '\\033]9;Build finished\\a'; printf '\\033]777;notify;Deploy;done in 3m\\033\\\\'; \
                  printf '\\033]99;i=1:d=0;Hel\\a\\033]99;i=1:d=0;lo\\a\\033]99;i=1:p=body;World\\a'; \
                  printf '\\033]9;4;1;42\\a'; printf 'sent'; sleep 5";
    let (pty, events) = spawn("/bin/sh", &["-c", script]);
    wait_until("the output", || pty.screen_text().contains("sent"));
    let notified: Vec<Notification> = drain(&events)
        .into_iter()
        .filter_map(|e| match e {
            PtyEvent::Notify(n) => Some(n),
            _ => None,
        })
        .collect();
    let notice = |title: &str, body: &str| Notification { title: title.into(), body: body.into() };
    assert_eq!(
        notified,
        [notice("", "Build finished"), notice("Deploy", "done in 3m"), notice("Hello", "World")]
    );
    assert_eq!(pty.progress(), Some(Progress { state: ProgressState::Normal, percent: 42 }));
    assert_eq!(pty.screen_text().trim_end(), "sent", "none of it shows on the screen");
}

/// How many bytes the reply to `OSC 7501 ; ?` and the primary device attributes make, and the
/// attributes as the test prints them (ESC as `E`): what the engine can do (sixel, selective
/// erase, color, the clipboard).
fn da1() -> (usize, &'static str) {
    (51, "E[?65;1;4;6;9;15;16;17;18;21;22;28;29;52c")
}

#[test]
fn the_program_status_reply_is_written_before_the_device_attributes_reply() {
    // A program asks `OSC 7501 ; ?` and sends `CSI c` right after it, and takes the device
    // attributes reply arriving first as "the protocol is not supported". The reply to the query
    // is written by the host before it writes the engine's reply to the `CSI c` that follows. Read back
    // with ESC shown as `E`: the 10 bytes of the first, then the 5 of the second.
    let (read, device_attributes) = da1();
    let script = format!(
        r#"stty raw -echo; printf '\033]7501;?\033\\\033[c'; r=$(dd bs=1 count={read} 2>/dev/null); stty sane; printf '%s' "$r" | tr '\033' E; sleep 5"#
    );
    // Nobody reads `events`: the replies come from the PTY thread alone.
    let (pty, _events) = spawn("/bin/sh", &["-c", &script]);
    wait_until("both replies, the status one first", || {
        pty.screen_text().contains(&format!(r"E]7501;?E\{device_attributes}"))
    });
}

#[test]
fn the_program_status_reply_comes_first_whichever_way_the_query_ends() {
    let (read, device_attributes) = da1();
    let script = format!(
        r#"stty raw -echo; printf '\033]7501;?\007\033[c'; r=$(dd bs=1 count={read} 2>/dev/null); stty sane; printf '%s' "$r" | tr '\033' E; sleep 5"#
    );
    let (pty, _events) = spawn("/bin/sh", &["-c", &script]);
    wait_until("both replies", || pty.screen_text().contains(&format!(r"E]7501;?E\{device_attributes}")));
}

/// The state, kind and message of what programs report, as the UI would read it.
fn reported(pty: &Pty) -> Option<(program_status::State, Option<program_status::Kind>, Option<String>)> {
    pty.program_status().summary().map(|s| (s.state, s.kind, s.message()))
}

use super::program_status;

#[test]
fn reports_reach_the_store_wake_the_ui_once_and_a_prompt_ends_the_work() {
    // The root works and a child asks (both before the UI looks), then a prompt begins and the child's
    // question turns out to have been answered: it reports done.
    let script = r#"printf '\033]7501;state=working:app=deploy\033\\'; printf '\033]7501;state=blocked:kind=permission:id=eu:msg=QXBwcm92ZT8=\007'; printf 'reported\n'; sleep 1; printf '\033]7501;state=done:id=eu\007\033]133;A\007'; printf 'prompt\n'; sleep 5"#;
    let (pty, events) = spawn("/bin/sh", &["-c", script]);
    wait_until("the reports", || pty.screen_text().contains("reported"));
    assert_eq!(
        reported(&pty),
        Some((
            program_status::State::Blocked,
            Some(program_status::Kind::Permission),
            Some("Approve?".into())
        ))
    );
    assert_eq!(pty.program_status().summary().and_then(|s| s.app).as_deref(), Some("deploy"), "inherited");
    // Two reports, one event: the UI looks once and reads the store.
    let wakes = |events: &super::PtyEvents| {
        drain(events).iter().filter(|e| matches!(e, PtyEvent::ProgramStatus)).count()
    };
    assert_eq!(wakes(&events), 1);

    wait_until("the prompt", || pty.screen_text().contains("prompt"));
    // The root's work is over with the prompt; the child's done record stays for the user to see.
    assert_eq!(reported(&pty), Some((program_status::State::Done, None, None)));
    assert_eq!(pty.program_status().len(), 1);
    assert_eq!(wakes(&events), 1, "and the UI is told again, as it had looked");
}

#[test]
fn a_flood_of_reports_costs_the_ui_one_event_and_the_store_256_records() {
    let script = r#"i=0; while [ $i -lt 3000 ]; do printf '\033]7501;state=working:id=r%s:msg=SGk=\007' $i; i=$((i+1)); done; printf flooded; sleep 5"#;
    let (pty, events) = spawn("/bin/sh", &["-c", script]);
    wait_until("the flood", || pty.screen_text().contains("flooded"));
    let wakes = drain(&events).iter().filter(|e| matches!(e, PtyEvent::ProgramStatus)).count();
    assert_eq!(wakes, 1, "the UI never looked in between");
    let store = pty.program_status();
    assert_eq!(store.len(), program_status::RECORDS_MAX);
    drop(store);
    // The least recently updated made room: the last one reported is there.
    assert_eq!(reported(&pty), Some((program_status::State::Working, None, Some("Hi".into()))));
}

#[test]
fn a_full_reset_ends_every_record_and_a_soft_reset_does_not() {
    // RIS (`ESC c`) wipes the screen too, so the script waits for the test between the two.
    let script = r#"printf '\033]7501;state=done:app=x\007'; sleep 1; printf '\033[!psoft'; sleep 2; printf '\033c'; sleep 5"#;
    let (pty, events) = spawn("/bin/sh", &["-c", script]);
    wait_until("the report", || !pty.program_status().is_empty());
    wait_until("the soft reset", || pty.screen_text().contains("soft"));
    assert!(!pty.program_status().is_empty(), "DECSTR keeps the records");
    wait_until("the full reset", || pty.program_status().is_empty());
    assert!(drain(&events).iter().any(|e| matches!(e, PtyEvent::ProgramStatus)));
}

#[test]
fn the_shells_exit_ends_the_work_but_not_what_finished() {
    let script = r#"printf '\033]7501;state=working:id=a\007\033]7501;state=done:id=b\007\033]7501;state=blocked:id=c\007'; exit 0"#;
    let (pty, events) = spawn("/bin/sh", &["-c", script]);
    wait_until("the exit", || matches!(events.try_recv(), Some(PtyEvent::Exited(_))));
    assert_eq!(reported(&pty), Some((program_status::State::Done, None, None)));
    assert_eq!(pty.program_status().len(), 1);
}

/// Whether a process exists (`kill -0`), without unsafe code in the test.
fn exists(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[test]
fn closing_a_tab_ends_a_shell_that_ignores_hangups() {
    // `trap '' HUP` (or `nohup`): the hangup alone never ends it.
    let (mut pty, _events) = spawn("/bin/sh", &["-c", "trap '' HUP; echo ready; sleep 30"]);
    // The kill comes after the grace period (3 s in the app); the test only needs it to come.
    pty.hangup_grace = Duration::from_millis(300);
    wait_until("the trap", || pty.screen_text().contains("ready"));
    let pid = pty.pid;
    assert!(exists(pid));
    drop(pty);
    let deadline = Instant::now() + Duration::from_secs(5);
    while exists(pid) {
        assert!(Instant::now() < deadline, "the shell outlived its tab");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// A shell with job control runs the program in the foreground in a process group of its own, so
/// the shell's group never reaches it; and once the tab is gone nobody reads the PTY, so a program
/// that keeps printing blocks on the full buffer, which keeps its shell from ever finishing exiting.
#[test]
fn closing_a_tab_ends_the_program_in_the_foreground_and_its_shell() {
    let dir = tempfile::tempdir().expect("temp dir");
    let pid_file = dir.path().join("job.pid");
    // The job ignores the hangup and floods the screen; `echo done` keeps the shell from
    // replacing itself with the job.
    let job = format!(
        r#"trap "" HUP; echo $$ > {}; while :; do echo flood flood flood flood; done"#,
        pid_file.display()
    );
    let script = format!("set -m; sh -c '{job}'; echo done");
    let (mut pty, _events) = spawn("/bin/sh", &["-c", &script]);
    pty.hangup_grace = Duration::from_millis(300);
    let mut job_pid = None;
    wait_until("the job's pid", || {
        job_pid = std::fs::read_to_string(&pid_file).ok().and_then(|text| text.trim().parse::<u32>().ok());
        job_pid.is_some()
    });
    let job_pid = job_pid.expect("the job started");
    let shell_pid = pty.pid;
    assert!(exists(shell_pid) && running(job_pid));
    // Without job control the job would share the shell's group and this would prove nothing.
    assert_ne!(ps("pgid", job_pid), ps("pgid", shell_pid), "the job leads a group of its own");
    drop(pty);
    // The test is the shell's parent, so the shell is gone once it is reaped. The job's parent was
    // the shell: whoever adopts it reaps it, if that is a container's init it may never, so a
    // zombie job is gone too.
    let deadline = Instant::now() + Duration::from_secs(8);
    while exists(shell_pid) || running(job_pid) {
        if Instant::now() >= deadline {
            let (shell_left, job_left) = (exists(shell_pid), running(job_pid));
            // A failed run must not leave a program printing into the void behind.
            for pid in [shell_pid, job_pid] {
                let _ = std::process::Command::new("kill").args(["-9", &pid.to_string()]).status();
            }
            panic!("left behind: shell {shell_pid} {shell_left}, job {job_pid} {job_left}");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// A column of `ps` for a process, empty when there is none.
fn ps(column: &str, pid: u32) -> String {
    let output = std::process::Command::new("ps")
        .args(["-o", &format!("{column}="), "-p", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .output();
    output.map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned()).unwrap_or_default()
}

/// Whether a process runs: it exists and is not a zombie waiting to be reaped.
fn running(pid: u32) -> bool {
    let state = ps("stat", pid);
    !state.is_empty() && !state.starts_with('Z')
}

#[test]
fn the_exit_code_is_reported() {
    let (_pty, events) = spawn("/bin/sh", &["-c", "exit 3"]);
    let mut code = None;
    wait_until("the exit", || match events.try_recv() {
        Some(PtyEvent::Exited(reported)) => {
            code = Some(reported);
            true
        }
        _ => false,
    });
    assert_eq!(code, Some(Some(3)));
}

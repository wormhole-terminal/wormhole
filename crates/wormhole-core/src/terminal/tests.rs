//! Real PTYs running small programs (`/bin/cat`, `/bin/sh -c …`), never the
//! user's shell. Nothing here polls the event receiver unless a test says so,
//! which is how a UI thread busy elsewhere looks to the terminal.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use alacritty_terminal::index::Line;
use alacritty_terminal::vte::ansi::Rgb;

use super::{
    BlockState, Direction, GridSize, Mark, Notification, Progress, ProgressState, PromptKind, Pty, PtyEvent,
    SpawnOptions,
};

const SIZE: GridSize = GridSize { cols: 80, rows: 24, cell_width: 8.0, cell_height: 16.0 };

fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn spawn(program: &str, args: &[&str]) -> (Pty, super::PtyEvents) {
    Pty::spawn_program(SIZE, &SpawnOptions::default(), program, args).expect("spawn")
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
fn a_color_reply_from_the_ui_comes_before_a_cursor_reply_asked_after_it() {
    // Programs ask for the background (OSC 11) and then the cursor position, and take the cursor
    // reply arriving first as "no color support"; the color reply then landed at the prompt.
    let script = r#"stty raw -echo; printf '\033]11;?\007\033[6n'; r=$(dd bs=1 count=30 2>/dev/null); stty sane; printf '%s' "$r" | tr '\033\007' EB; sleep 5"#;
    let (pty, events) = spawn("/bin/sh", &["-c", script]);
    let deadline = Instant::now() + Duration::from_secs(10);
    let (format, reply) = loop {
        assert!(Instant::now() < deadline, "timed out waiting for the color request");
        match events.try_recv() {
            Some(PtyEvent::ColorRequest(257, format, reply)) => break (format, reply),
            Some(_) => {}
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    // A busy UI: the cursor reply is long done on the PTY thread by now.
    std::thread::sleep(Duration::from_millis(200));
    reply.send(format(Rgb { r: 1, g: 2, b: 3 }).into_bytes());
    wait_until("both replies", || pty.screen_text().contains("E]11;rgb:0101/0202/0303BE[1;1R"));
}

#[test]
fn a_query_the_ui_drops_lets_the_replies_behind_it_go() {
    let script = r#"stty raw -echo; printf '\033]11;?\007\033[6n'; r=$(dd bs=1 count=6 2>/dev/null); stty sane; printf '%s' "$r" | tr '\033' E; sleep 5"#;
    let (pty, events) = spawn("/bin/sh", &["-c", script]);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < deadline, "timed out waiting for the color request");
        match events.try_recv() {
            // Dropped unanswered, as when the view is gone.
            Some(PtyEvent::ColorRequest(..)) => break,
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

/// UI wakeups for streaming output: alacritty's wakeups (each one an event in
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

#[test]
fn a_project_env_cannot_redirect_the_hooks() {
    let mut options = SpawnOptions { tab_id: "real-tab".into(), ..SpawnOptions::default() };
    options.env.insert("WORMHOLE_TAB_ID".into(), "spoofed".into());
    options.env.insert("WORMHOLE_SOCKET".into(), "/tmp/elsewhere.sock".into());
    options.env.insert("PROJECT_FLAG".into(), "on".into());
    let script = r#"printf '[%s] [%s] [%s] [%s %s]\n' "$WORMHOLE_TAB_ID" "$PROJECT_FLAG" "$WORMHOLE_SOCKET" "$TERM_PROGRAM" "$TERM_PROGRAM_VERSION"; sleep 5"#;
    let (pty, _events) = Pty::spawn_program(SIZE, &options, "/bin/sh", &["-c", script]).expect("spawn");
    wait_until("the output", || pty.screen_text().contains("[real-tab] [on]"));
    let screen = pty.screen_text();
    assert!(!screen.contains("elsewhere"), "{screen}");
    let version = env!("CARGO_PKG_VERSION");
    assert!(screen.contains(&format!("[Wormhole {version}]")), "{screen}");
}

/// Everything `events` has queued right now, in order.
fn drain(events: &super::PtyEvents) -> Vec<PtyEvent> {
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
    let options = SpawnOptions { scrollback_lines: 1000, ..SpawnOptions::default() };
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
fn inline_images_land_at_the_cursor_and_decode_off_the_thread() {
    // A 2×1 RGB picture by kitty's protocol (six zero bytes), then a one-column Sixel on the next
    // row: a placeholder cell each in the grid, the pixels in the store.
    let script = "printf '\\033_Ga=T,f=24,s=2,v=1,i=7,q=2;AAAAAAAA\\033\\\\'; printf '\\n\\033Pq#1;2;100;0;0~\\033\\\\'; printf 'done'; sleep 5";
    let (pty, _events) = spawn("/bin/sh", &["-c", script]);
    wait_until("the output", || pty.screen_text().contains("done"));
    let screen = pty.screen_text();
    let placeholders = screen.chars().filter(|c| *c == super::images::PLACEHOLDER).count();
    assert_eq!(placeholders, 2, "{screen}");
    assert_eq!(screen.lines().nth(2).map(str::trim), Some("done"), "under the Sixel:\n{screen}");
    wait_until("the decode", || pty.images().get(7).is_some());
    let mut store = pty.images();
    assert_eq!(store.dimensions(7), Some((2, 1)));
    assert_eq!(store.get(7).map(|image| image.rgba.len()), Some(8));
    assert_eq!(store.len(), 2, "the Sixel is decoded in place");
    drop(store);
    // Placeholders aren't text.
    pty.select_all();
    assert!(!pty.selection_text().unwrap_or_default().contains(super::images::PLACEHOLDER));
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

/// The variables that configure an instance (its data folder, its devctl folder, background mode, …)
/// stay with the instance: `cargo run` in a test instance's tab would otherwise start a second
/// instance on the first one's data and take commands meant for it. A process's environment can't be
/// changed from inside a test, so the check runs again in a child process that has them all.
#[test]
fn a_shell_does_not_inherit_the_instances_own_settings() {
    const CHILD: &str = "WORMHOLE_SCRUB_TEST_CHILD";
    const TEST: &str = "terminal::tests::a_shell_does_not_inherit_the_instances_own_settings";
    if std::env::var_os(CHILD).is_none() {
        let exe = std::env::current_exe().expect("finds the test binary");
        let mut command = std::process::Command::new(exe);
        command.args([TEST, "--exact", "--test-threads=1"]).env(CHILD, "1");
        for var in super::process::INSTANCE_VARS {
            command.env(var, "/leaked");
        }
        // Not the instance's: a person's own log level reaches their shell.
        command.env("WORMHOLE_LOG", "debug");
        let output = command.output().expect("runs the test binary");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && stdout.contains("1 passed"),
            "{TEST} failed in its child process:\n{stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let options = SpawnOptions { tab_id: "real-tab".into(), ..SpawnOptions::default() };
    let shows: String = super::process::INSTANCE_VARS
        .iter()
        .chain(&["WORMHOLE_LOG", "WORMHOLE_TAB_ID"])
        .map(|var| format!(" \"{var}=${{{var}-unset}}\""))
        .collect();
    let script = format!("printf '%s\\n'{shows}; sleep 5");
    let (pty, _events) = Pty::spawn_program(SIZE, &options, "/bin/sh", &["-c", &script]).expect("spawn");
    wait_until("the output", || pty.screen_text().contains("WORMHOLE_TAB_ID=real-tab"));
    let screen = pty.screen_text();
    for var in super::process::INSTANCE_VARS {
        assert!(screen.contains(&format!("{var}=unset")), "{var} reached the shell:\n{screen}");
    }
    assert!(screen.contains("WORMHOLE_LOG=debug"), "a setting that isn't the instance's stays:\n{screen}");
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

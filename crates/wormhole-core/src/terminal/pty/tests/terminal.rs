//! What a program sees of its terminal: its size, its session, the line discipline, the keys it
//! is sent.

use crate::terminal::WindowSize;
use crate::terminal::pty::{discipline, open};

use super::run;

#[test]
fn the_window_size_reaches_the_program_and_a_resize_signals_it() {
    let script = "trap 'echo winch:$(stty size)' WINCH; stty size; echo ready; while :; do sleep 0.05; done";
    let run = run("/bin/sh", &["-c", script]);
    run.wait_for("24 80\r\n");
    run.wait_for("ready");
    run.control.resize(WindowSize { num_lines: 30, num_cols: 100, cell_width: 8, cell_height: 16 });
    run.wait_for("winch:30 100");
}

#[test]
fn the_window_size_in_pixels_is_cells_times_cell_size() {
    let ws = open::winsize(WindowSize { num_lines: 24, num_cols: 80, cell_width: 9, cell_height: 18 });
    assert_eq!((ws.ws_row, ws.ws_col, ws.ws_xpixel, ws.ws_ypixel), (24, 80, 720, 432));
    // Huge cells saturate instead of wrapping around.
    let ws = open::winsize(WindowSize { num_lines: 500, num_cols: 500, cell_width: 500, cell_height: 500 });
    assert_eq!((ws.ws_xpixel, ws.ws_ypixel), (u16::MAX, u16::MAX));
}

#[test]
fn the_program_leads_a_session_with_the_terminal_as_its_controlling_terminal() {
    // `ps` says the shell's process group and the terminal's foreground group; `/dev/tty` opens
    // only with a controlling terminal, and `tty` names the device the streams are.
    let script = "ps -o pid=,pgid=,tpgid= -p $$; tty; (: < /dev/tty) && echo has-ctty";
    let run = run("/bin/sh", &["-c", script]);
    let text = run.wait_for("has-ctty");
    let mut lines = text.lines();
    let ids: Vec<u32> =
        lines.next().unwrap_or("").split_whitespace().filter_map(|id| id.parse().ok()).collect();
    assert_eq!(ids, [run.pid, run.pid, run.pid], "pid, process group and foreground group: {text}");
    assert!(lines.next().is_some_and(|line| line.starts_with("/dev/tty")), "{text}");
}

#[test]
fn the_terminal_expects_utf8_and_is_sane() {
    let run = run("/bin/sh", &["-c", "stty -a; echo end-of-stty"]);
    let text = run.wait_for("end-of-stty");
    let flags: Vec<&str> = text.split_whitespace().collect();
    for set in ["iutf8", "icrnl", "opost", "onlcr", "isig", "icanon", "echo", "cread", "cs8", "hupcl"] {
        assert!(flags.contains(&set), "{set} is not set:\n{text}");
    }
    for unset in ["-ignbrk", "-ixoff", "-parenb", "-echonl", "-noflsh", "-tostop", "-istrip"] {
        assert!(flags.contains(&unset), "{unset} is not off:\n{text}");
    }
}

#[test]
fn sane_makes_a_hostile_line_discipline_sane() {
    #[allow(unsafe_code)]
    // SAFETY: an all-zero termios is a valid value of the plain C struct.
    let mut termios: libc::termios = unsafe { std::mem::zeroed() };
    termios.c_iflag = !0;
    termios.c_oflag = !0;
    termios.c_cflag = !0;
    termios.c_lflag = !0;
    discipline::sane(&mut termios);
    assert_ne!(termios.c_iflag & libc::IUTF8, 0);
    assert_eq!(termios.c_iflag & (libc::IGNBRK | libc::ISTRIP | libc::IXOFF | libc::INLCR), 0);
    assert_eq!(termios.c_cflag & libc::CSIZE, libc::CS8);
    assert_eq!(termios.c_cflag & (libc::PARENB | libc::CSTOPB), 0);
    assert_eq!(termios.c_lflag & (libc::NOFLSH | libc::TOSTOP | libc::ECHONL), 0);
    assert_eq!(termios.c_cc[libc::VERASE], 0x7f);
}

#[test]
fn ctrl_c_interrupts_the_program_in_the_foreground() {
    // The line discipline turns 0x03 into SIGINT for the terminal's foreground group, which
    // takes the sleep with it; the shell traps it and goes on.
    let run = run("/bin/sh", &["-c", "trap 'echo interrupted' INT; echo ready; sleep 30; echo after"]);
    run.wait_for("ready");
    run.control.input(b"\x03".as_slice().into());
    run.wait_for("after");
    assert!(run.text().contains("interrupted"));
}

#[test]
fn input_reaches_the_program_and_comes_back_echoed_and_printed() {
    let run = run("/bin/cat", &[]);
    run.control.input(b"ping\n".as_slice().into());
    // Once as the terminal echoes the typed line, once as cat prints it.
    run.wait_for("ping\r\nping\r\n");
}

#[test]
fn input_given_before_the_program_reads_waits_for_it() {
    let run = run("/bin/sh", &["-c", "sleep 0.3; read line; echo got:$line"]);
    run.control.input(b"early\n".as_slice().into());
    run.wait_for("got:early");
}

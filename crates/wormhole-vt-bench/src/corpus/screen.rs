//! Programs that address the screen: a redrawing full-screen program and scrolling regions.

use super::Out;

struct Process {
    pid: u32,
    user: &'static str,
    cpu: f32,
    mem: f32,
    command: String,
}

/// An htop-like program: enter the alternate screen, draw everything, then keep redrawing the
/// table with cursor addressing; every other frame only the rows whose numbers changed.
pub(super) fn tui_redraw(out: &mut Out, target: usize) {
    let (cols, rows) = (out.cols, out.rows);
    out.text("\x1b[?1049h\x1b[?25l\x1b[2J");
    let mut processes: Vec<Process> = (0..rows.saturating_sub(10))
        .map(|i| Process {
            pid: 1000 + i as u32 * 7,
            user: ["root", "user", "daemon", "nobody"][i % 4],
            cpu: (out.rng.below(1000) as f32) / 10.0,
            mem: (out.rng.below(500) as f32) / 10.0,
            command: out.line_between(8, 40),
        })
        .collect();
    let mut frame = 0usize;
    while out.len() < target {
        let full = frame.is_multiple_of(4);
        out.goto(1, 1);
        for core in 0..4 {
            let load = out.rng.range(0, 40);
            out.goto(1 + core, 1);
            out.sgr("1;36");
            out.text(&format!("{core:>3}"));
            out.sgr("0");
            out.text("[");
            out.sgr("32");
            out.text(&"|".repeat(load));
            out.sgr("31");
            let hot = out.rng.range(0, 8);
            out.text(&"|".repeat(hot));
            out.sgr("0");
            out.text(&" ".repeat(10));
            out.text(&format!("{:>5.1}%]\x1b[K", load as f32 * 2.5));
        }
        if full {
            out.goto(6, 1);
            out.sgr("30;42");
            out.text("  PID USER       CPU%  MEM%   Command");
            out.csi("K");
            out.sgr("0");
        }
        let selected = frame % processes.len().max(1);
        for (i, p) in processes.iter_mut().enumerate() {
            if full || out.rng.one_in(4) {
                p.cpu =
                    ((p.cpu + out.rng.below(60) as f32 / 10.0 - 3.0).clamp(0.0, 99.9) * 10.0).round() / 10.0;
                p.mem =
                    ((p.mem + out.rng.below(20) as f32 / 10.0 - 1.0).clamp(0.0, 99.9) * 10.0).round() / 10.0;
                out.goto(7 + i, 1);
                if i == selected {
                    out.sgr("30;46");
                }
                out.text(&format!("{:>5} {:<9} {:>5.1} {:>5.1}   ", p.pid, p.user, p.cpu, p.mem));
                if p.cpu > 50.0 {
                    out.sgr("1;31");
                }
                out.text(&p.command[..p.command.len().min(cols.saturating_sub(34))]);
                out.sgr("0");
                out.csi("K");
            }
        }
        out.goto(rows, 1);
        out.sgr("30;46");
        out.text("F1");
        out.sgr("0;7");
        out.text("Help  ");
        out.sgr("0;30;46");
        out.text("F2");
        out.sgr("0;7");
        out.text("Setup ");
        out.sgr("0;30;46");
        out.text("F10");
        out.sgr("0;7");
        out.text("Quit");
        out.csi("K");
        out.sgr("0");
        frame += 1;
    }
    // The program is still running when the stream ends: the table stays on the screen.
}

/// A pager-like screen: a header and a footer that stay, a middle that scrolls inside DECSTBM
/// margins, with lines inserted and deleted, the region scrolled by SU and SD and reverse
/// index at its top margin.
pub(super) fn scroll_region(out: &mut Out, target: usize) {
    let rows = out.rows;
    out.text("\x1b[2J\x1b[H");
    out.sgr("7");
    out.text("== log viewer ==");
    out.csi("K");
    out.sgr("0");
    let mut bottom = rows - 1;
    let mut top = 2;
    out.csi(&format!("{top};{bottom}r"));
    out.goto(bottom, 1);
    let mut n = 0u64;
    while out.len() < target {
        n += 1;
        let len = out.rng.range(10, 110);
        let line = out.line_of(len);
        out.text(&format!("{n:>6} {line}\r\n"));
        match out.rng.below(40) {
            0 => {
                out.goto(top + 3, 1);
                {
                    let n = out.rng.range(1, 4);
                    out.csi(&format!("{n}L"));
                };
                out.goto(bottom, 1);
            }
            1 => {
                out.goto(top + 3, 1);
                {
                    let n = out.rng.range(1, 4);
                    out.csi(&format!("{n}M"));
                };
                out.goto(bottom, 1);
            }
            2 => {
                let n = out.rng.range(1, 5);
                out.csi(&format!("{n}S"));
            }
            3 => {
                let n = out.rng.range(1, 5);
                out.csi(&format!("{n}T"));
            }
            4 | 5 => {
                out.goto(top, 1);
                for _ in 0..out.rng.range(1, 4) {
                    out.text("\x1bM");
                }
                out.goto(bottom, 1);
            }
            6 => {
                // The status in the footer, drawn without disturbing the cursor.
                out.text("\x1b7");
                out.goto(rows, 1);
                out.sgr("7");
                out.text(&format!(" line {n} "));
                out.csi("K");
                out.sgr("0");
                out.text("\x1b8");
            }
            7 => {
                // Another region: the top half, then back.
                out.csi("r");
                top = out.rng.range(2, 6);
                bottom = out.rng.range(rows - 8, rows - 1);
                out.csi(&format!("{top};{bottom}r"));
                out.goto(bottom, 1);
            }
            _ => {}
        }
    }
    out.csi("r");
}

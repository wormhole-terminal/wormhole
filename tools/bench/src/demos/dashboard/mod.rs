//! A busy, made-up system monitor in the style of htop and btop: per-core CPU bars with gradients,
//! memory gauges, network graphs of block characters that scroll, a process table that re-sorts every
//! frame, and a log pane of agent and build lines, all in rounded boxes. Every value comes from smooth
//! noise over time, so it looks alive and repeats exactly. It stresses what real TUIs do: many small
//! colored text runs, box drawing, partial blocks, and a screen that changes a little everywhere.

mod data;

use std::collections::VecDeque;
use std::fmt::Write;

use crate::canvas::{Cells, Rgb, Rng};
use crate::demo::{Demo, Frame};
use data::*;

const SCREEN: Rgb = Rgb(11, 13, 20);
const PANEL: Rgb = Rgb(15, 18, 27);
const TRACK: Rgb = Rgb(30, 34, 48);
const TEXT: Rgb = Rgb(208, 214, 230);
const DIM: Rgb = Rgb(104, 112, 138);
/// Seconds between graph samples; the graphs scroll one column per sample.
const SAMPLE: f64 = 0.1;
const BLOCKS: [char; 8] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇'];
const PARTIAL: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];

pub struct Dashboard {
    cells: Cells,
    procs: Vec<Proc>,
    order: Vec<usize>,
    log: VecDeque<Line>,
    next_log: f64,
    next_sample: f64,
    /// Newest last: network in and out, average CPU and memory, all 0..1.
    rx: VecDeque<f64>,
    tx: VecDeque<f64>,
    cpu: VecDeque<f64>,
    ram: VecDeque<f64>,
    rng: Rng,
    scratch: String,
}

impl Default for Dashboard {
    fn default() -> Self {
        let mut rng = Rng::new(0xda5b_0a7d);
        let procs: Vec<Proc> = (0..64)
            .map(|i| {
                let (cmd, user, weight) = PROCS[i % PROCS.len()];
                Proc {
                    pid: 100 + rng.below(90000) as u32,
                    user,
                    cmd,
                    weight: weight * rng.range(0.5, 1.2) / (1.0 + (i / PROCS.len()) as f64 * 1.5),
                    mem: rng.range(0.1, 6.0) * if weight > 0.3 { 2.0 } else { 0.5 },
                    seconds: rng.range(10.0, 90000.0),
                    seed: i as u32 * 7 + 3,
                }
            })
            .collect();
        let order = (0..procs.len()).collect();
        let mut d = Self {
            cells: Cells::new(),
            procs,
            order,
            log: VecDeque::new(),
            next_log: 0.0,
            next_sample: 0.0,
            rx: VecDeque::new(),
            tx: VecDeque::new(),
            cpu: VecDeque::new(),
            ram: VecDeque::new(),
            rng,
            scratch: String::new(),
        };
        // Start with history on screen: the last 80 seconds of graphs and a page of log.
        for k in (1..=800).rev() {
            d.sample(-(k as f64) * SAMPLE);
        }
        for k in (1..=80).rev() {
            let line = log_line(&mut d.rng, -(k as f64) * 0.4);
            d.log.push_back(line);
        }
        d
    }
}

fn push(q: &mut VecDeque<f64>, v: f64) {
    q.push_back(v);
    while q.len() > 1200 {
        q.pop_front();
    }
}

impl Dashboard {
    fn sample(&mut self, t: f64) {
        let (rx, tx) = net(t);
        push(&mut self.rx, rx);
        push(&mut self.tx, tx);
        push(&mut self.cpu, avg_cpu(t));
        // Scaled to its own range, or the graph would be a flat slab.
        push(&mut self.ram, ((mem(t) - 0.35) / 0.55).clamp(0.0, 1.0));
    }

    fn txt(&mut self, x: usize, y: usize, max: usize, s: &str, color: Rgb, back: Rgb) -> usize {
        let mut n = 0;
        for ch in s.chars().take(max) {
            self.cells.put(x + n, y, ch, color, back);
            n += 1;
        }
        n
    }

    fn fill(&mut self, x: usize, y: usize, w: usize, h: usize, back: Rgb) {
        for r in y..y + h {
            for c in x..x + w {
                self.cells.put(c, r, ' ', TEXT, back);
            }
        }
    }

    /// A rounded box with a title in its top edge; returns the inside as (x, y, w, h).
    fn boxed(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        title: &str,
        edge: Rgb,
    ) -> Option<(usize, usize, usize, usize)> {
        if w < 4 || h < 3 || x + w > self.cells.cols || y + h > self.cells.rows {
            return None;
        }
        self.fill(x, y, w, h, PANEL);
        for c in 1..w - 1 {
            self.cells.put(x + c, y, '─', edge, PANEL);
            self.cells.put(x + c, y + h - 1, '─', edge, PANEL);
        }
        for r in 1..h - 1 {
            self.cells.put(x, y + r, '│', edge, PANEL);
            self.cells.put(x + w - 1, y + r, '│', edge, PANEL);
        }
        for (cx, cy, ch) in
            [(x, y, '╭'), (x + w - 1, y, '╮'), (x, y + h - 1, '╰'), (x + w - 1, y + h - 1, '╯')]
        {
            self.cells.put(cx, cy, ch, edge, PANEL);
        }
        let label = format!(" {title} ");
        self.txt(x + 2, y, w.saturating_sub(4), &label, TEXT, PANEL);
        Some((x + 2, y + 1, w - 4, h - 2))
    }

    /// A horizontal gauge in eighths of a cell, colored by `color(position)`.
    fn bar(&mut self, x: usize, y: usize, w: usize, frac: f64, color: impl Fn(f64) -> Rgb) {
        let filled = frac.clamp(0.0, 1.0) * w as f64;
        for c in 0..w {
            let cover = (filled - c as f64).clamp(0.0, 1.0);
            let k = color((c as f64 + 0.5) / w as f64);
            let (ch, f) = if cover > 0.99 { ('█', k) } else { (PARTIAL[(cover * 8.0) as usize], k) };
            self.cells.put(x + c, y, ch, f, TRACK);
        }
    }

    /// Filled graph of the newest `w` samples, `h` rows tall, newest at the right.
    fn area(
        &mut self,
        (x, y, w, h): (usize, usize, usize, usize),
        series: &VecDeque<f64>,
        base: Rgb,
        tip: Rgb,
    ) {
        for c in 0..w {
            let k = (series.len() + c) as isize - w as isize;
            let v = if k >= 0 { series[k as usize] } else { 0.0 };
            let eighths = (v * h as f64 * 8.0).round() as isize;
            for r in 0..h {
                let rem = eighths - r as isize * 8;
                let ch = if rem >= 8 {
                    '█'
                } else if rem <= 0 {
                    ' '
                } else {
                    BLOCKS[rem as usize]
                };
                let color = base.lerp(tip, (r + 1) as f64 / h as f64);
                self.cells.put(x + c, y + h - 1 - r, ch, color, PANEL);
            }
        }
    }

    fn cpu_box(&mut self, x: usize, y: usize, w: usize, h: usize, t: f64) {
        let Some((ix, iy, iw, ih)) = self.boxed(x, y, w, h, "CPU", Rgb(96, 190, 240)) else { return };
        let crows = ih.min(8);
        let ncols = CORES.div_ceil(crows).min(4);
        let cw = iw / ncols;
        for i in 0..(ncols * crows).min(CORES) {
            let (col, row) = (i / crows, i % crows);
            let (cx, cy) = (ix + col * cw, iy + row);
            let load = core_load(i, t);
            self.scratch.clear();
            let _ = write!(self.scratch, "{i:>2}");
            let label = std::mem::take(&mut self.scratch);
            self.txt(cx, cy, 2, &label, DIM, PANEL);
            let bw = cw.saturating_sub(10).max(3);
            self.bar(cx + 3, cy, bw, load, heat);
            let pct = format!("{:>4.0}%", load * 100.0);
            self.txt(cx + 3 + bw, cy, 5, &pct, heat(load), PANEL);
            self.scratch = label;
        }
        if ih >= crows + 3 {
            let gy = iy + crows + 1;
            let gh = ih - crows - 1;
            self.txt(ix, gy - 1, iw, "total", DIM, PANEL);
            let cpu = std::mem::take(&mut self.cpu);
            self.area((ix, gy, iw, gh), &cpu, Rgb(30, 100, 130), Rgb(120, 230, 255));
            self.cpu = cpu;
        }
    }

    fn mem_box(&mut self, x: usize, y: usize, w: usize, h: usize, t: f64) {
        let Some((ix, iy, iw, ih)) = self.boxed(x, y, w, h, "MEM", Rgb(190, 130, 250)) else { return };
        let used = mem(t);
        let swap = (0.1 + 0.1 * fbm(t * 0.03, 31)).clamp(0.0, 1.0);
        let bw = iw.saturating_sub(18).max(3);
        for (row, label, frac, total) in [(0, "Mem", used, 64.0), (1, "Swp", swap, 8.0)] {
            if row >= ih {
                break;
            }
            self.txt(ix, iy + row, 3, label, DIM, PANEL);
            let seg = [
                (0.7 * frac, Rgb(86, 214, 130)),
                (0.15 * frac, Rgb(90, 150, 250)),
                (0.15 * frac, Rgb(244, 200, 72)),
            ];
            self.bar(ix + 4, iy + row, bw, frac, |p| {
                let mut acc = 0.0;
                for (s, c) in seg {
                    acc += s;
                    if p * frac.max(1e-6) <= acc {
                        return c;
                    }
                }
                seg[2].1
            });
            let s = format!(" {:>4.1}G/{:.0}G", frac * total, total);
            self.txt(ix + 4 + bw, iy + row, 12, &s, TEXT, PANEL);
        }
        let run = 2 + (busy(t) * 9.0) as u32;
        let lines = [
            format!(
                "Tasks {} · {} thr · {run} run",
                310 + (t * 0.7) as u32 % 9,
                1180 + (t * 3.1) as u32 % 40
            ),
            format!(
                "Load  {:.2} {:.2} {:.2}",
                1.0 + 6.0 * busy(t),
                1.5 + 4.0 * busy(t - 5.0),
                1.8 + 3.0 * busy(t - 15.0)
            ),
            format!(
                "Up    3d {:02}:{:02}:{:02}",
                (4 + t as u64 / 3600) % 24,
                (12 + t as u64 / 60) % 60,
                (55 + t as u64) % 60
            ),
        ];
        for (k, l) in lines.iter().enumerate() {
            if 2 + k < ih {
                self.txt(ix, iy + 2 + k, iw, l, TEXT, PANEL);
            }
        }
        if ih >= 7 {
            let gy = iy + 5;
            let ram = std::mem::take(&mut self.ram);
            self.area((ix, gy, iw, ih - 5), &ram, Rgb(60, 40, 110), Rgb(200, 150, 255));
            self.ram = ram;
        }
    }

    fn net_box(&mut self, x: usize, y: usize, w: usize, h: usize, t: f64) {
        let Some((ix, iy, iw, ih)) = self.boxed(x, y, w, h, "NET", Rgb(100, 220, 160)) else { return };
        let (rx, tx) = net(t);
        let half = (ih / 2).max(1);
        let lw = if iw >= 28 { 12 } else { 0 };
        for (k, (name, v, max, series, base, tip)) in [
            ("▼ RX", rx, 220.0, std::mem::take(&mut self.rx), Rgb(24, 100, 70), Rgb(120, 250, 190)),
            ("▲ TX", tx, 60.0, std::mem::take(&mut self.tx), Rgb(110, 60, 30), Rgb(255, 180, 90)),
        ]
        .into_iter()
        .enumerate()
        {
            let gy = iy + k * half;
            let gh = if k == 0 { half } else { ih.saturating_sub(half).max(1) };
            if gy + gh <= iy + ih {
                if lw > 0 {
                    self.txt(ix, gy, lw, name, tip, PANEL);
                    if gh >= 2 {
                        let s = format!("{:.1} MB/s", v * max);
                        self.txt(ix, gy + 1, lw, &s, TEXT, PANEL);
                    }
                }
                self.area((ix + lw, gy, iw - lw, gh), &series, base, tip);
            }
            if k == 0 { self.rx = series } else { self.tx = series }
        }
    }

    fn proc_box(&mut self, x: usize, y: usize, w: usize, h: usize, t: f64) {
        let Some((ix, iy, iw, ih)) = self.boxed(x, y, w, h, "PROCESSES", Rgb(244, 200, 100)) else { return };
        let cpus: Vec<f64> = self.procs.iter().map(|p| p.cpu(t)).collect();
        self.order.sort_by(|&a, &b| cpus[b].partial_cmp(&cpus[a]).unwrap_or(std::cmp::Ordering::Equal));
        let (show_user, show_time, show_mem) = (iw >= 56, iw >= 44, iw >= 30);
        let head_bg = Rgb(40, 74, 50);
        self.fill(ix, iy, iw, 1, head_bg);
        let mut hx = ix;
        let mut head = |this: &mut Self, s: &str, wd: usize| {
            this.txt(hx, iy, wd, s, Rgb(220, 240, 224), head_bg);
            hx += wd;
        };
        head(self, "   PID ", 7);
        if show_user {
            head(self, "USER      ", 10);
        }
        head(self, "  CPU% ", 7);
        if show_mem {
            head(self, " MEM% ", 6);
        }
        if show_time {
            head(self, "    TIME+ ", 10);
        }
        head(self, "Command", 7);
        let used = 7
            + if show_user { 10 } else { 0 }
            + 7
            + if show_mem { 6 } else { 0 }
            + if show_time { 10 } else { 0 };
        for row in 0..ih.saturating_sub(1) {
            let Some(&pi) = self.order.get(row) else { break };
            let p = &self.procs[pi];
            let (cpu, mem_pct) = (cpus[pi], p.mem);
            let secs = p.seconds + t * cpu / 100.0;
            let (user, cmd, pid) = (p.user, p.cmd, p.pid);
            let back = if row == 0 {
                Rgb(34, 54, 86)
            } else if row % 2 == 1 {
                Rgb(18, 21, 32)
            } else {
                PANEL
            };
            let ry = iy + 1 + row;
            self.fill(ix, ry, iw, 1, back);
            let mut cx = ix;
            let s = format!("{pid:>6} ");
            cx += self.txt(cx, ry, 7, &s, DIM, back);
            if show_user {
                let s = format!("{:<10}", &user[..user.len().min(9)]);
                cx += self.txt(cx, ry, 10, &s, Rgb(150, 190, 240), back);
            }
            let s = format!("{cpu:>6.1} ");
            cx += self.txt(cx, ry, 7, &s, heat((cpu / 100.0).min(1.0)), back);
            if show_mem {
                let s = format!("{mem_pct:>5.1} ");
                cx += self.txt(cx, ry, 6, &s, TEXT, back);
            }
            if show_time {
                let s = format!(
                    "{:>3}:{:02}.{:02} ",
                    secs as u64 / 60 % 1000,
                    secs as u64 % 60,
                    (secs.fract() * 100.0) as u32
                );
                cx += self.txt(cx, ry, 10, &s, DIM, back);
            }
            let bright =
                if cmd.starts_with("rustc") || cmd.starts_with("cargo") { Rgb(120, 230, 160) } else { TEXT };
            self.txt(cx, ry, iw.saturating_sub(used), cmd, bright, back);
        }
    }

    fn log_box(&mut self, x: usize, y: usize, w: usize, h: usize, t: f64) {
        let Some((ix, iy, iw, ih)) = self.boxed(x, y, w, h, "AGENT LOG", Rgb(242, 130, 160)) else { return };
        let n = self.log.len();
        for row in 0..ih.min(n) {
            let line = &self.log[n - 1 - row];
            let fresh = (1.0 - (t - line.born) / 0.5).clamp(0.0, 1.0);
            let secs = (14 * 3600 + 3 * 60 + 22 + line.born as i64).max(0) as u64;
            let clock = format!("{:02}:{:02}:{:02} ", secs / 3600 % 24, secs / 60 % 60, secs % 60);
            let (name, color) = SOURCES[line.src];
            let tag = format!("{name:<7}");
            let msg_color = match line.level {
                1 => Rgb(244, 200, 72),
                2 => Rgb(250, 100, 96),
                _ => TEXT,
            }
            .lerp(Rgb::WHITE, fresh * 0.6);
            let ry = iy + ih - 1 - row;
            let msg = line.msg.clone();
            let mut cx = ix;
            cx += self.txt(cx, ry, iw, &clock, DIM, PANEL);
            cx += self.txt(cx, ry, iw.saturating_sub(cx - ix), &tag, color, PANEL);
            self.txt(cx, ry, iw.saturating_sub(cx - ix), &msg, msg_color, PANEL);
        }
    }

    fn header(&mut self, t: f64) {
        let cols = self.cells.cols;
        let back = Rgb(22, 26, 40);
        self.fill(0, 0, cols, 1, back);
        let secs = 14 * 3600 + 3 * 60 + 22 + t as u64;
        let s = format!(
            " ◉ wormhole-top  {:02}:{:02}:{:02}  up 3d {:02}:{:02}  load {:.2}  cpu {:>3.0}%  mem {:>3.0}% ",
            secs / 3600 % 24,
            secs / 60 % 60,
            secs % 60,
            (4 + t as u64 / 3600) % 24,
            (12 + t as u64 / 60) % 60,
            1.0 + 6.0 * busy(t),
            avg_cpu(t) * 100.0,
            mem(t) * 100.0,
        );
        self.txt(0, 0, cols, &s, Rgb(150, 210, 250), back);
    }
}

impl Demo for Dashboard {
    fn frame(&mut self, f: &mut Frame) {
        self.cells.fit(f.size.cols, f.size.rows);
        let (cols, rows) = (self.cells.cols, self.cells.rows);
        self.cells.clear(SCREEN);
        if cols == 0 || rows == 0 {
            return;
        }
        let t = f.t;
        while t >= self.next_sample {
            let at = self.next_sample;
            self.sample(at);
            self.next_sample += SAMPLE;
        }
        while t >= self.next_log {
            let line = log_line(&mut self.rng, self.next_log);
            self.log.push_back(line);
            while self.log.len() > 300 {
                self.log.pop_front();
            }
            self.next_log += 0.1 + (1.0 - 0.7 * busy(t)) * 0.7 * self.rng.f();
        }

        self.header(t);
        let avail = rows - 1;
        if cols < 40 || avail < 7 {
            // Too small for boxes: the cores, one per row.
            for i in 0..CORES.min(avail) {
                let load = core_load(i, t);
                let label = format!("{i:>2}");
                self.txt(0, 1 + i, 2, &label, DIM, SCREEN);
                let bw = cols.saturating_sub(9).max(1);
                self.bar(3, 1 + i, bw, load, heat);
                let pct = format!("{:>4.0}%", load * 100.0);
                self.txt(3 + bw, 1 + i, 5, &pct, heat(load), SCREEN);
            }
        } else if cols >= 110 {
            let top = (rows / 3).clamp(6, 18).min(avail - 4);
            let (w1, w2) = (cols * 45 / 100, cols * 25 / 100);
            self.cpu_box(0, 1, w1, top, t);
            self.mem_box(w1, 1, w2, top, t);
            self.net_box(w1 + w2, 1, cols - w1 - w2, top, t);
            let (lw, y, h) = (cols * 58 / 100, 1 + top, avail - top);
            self.proc_box(0, y, lw, h, t);
            self.log_box(lw, y, cols - lw, h, t);
        } else {
            let top = (rows / 4).clamp(6, 10).min(avail);
            self.cpu_box(0, 1, cols, top, t);
            let mut y = 1 + top;
            if rows >= 20 {
                let mh = if rows >= 40 { 10 } else { 6 };
                self.mem_box(0, y, cols / 2, mh, t);
                self.net_box(cols / 2, y, cols - cols / 2, mh, t);
                y += mh;
            }
            if rows > y + 3 {
                self.proc_box(0, y, cols / 2, rows - y, t);
                self.log_box(cols / 2, y, cols - cols / 2, rows - y, t);
            }
        }
        self.cells.render(f.out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::Size;

    #[test]
    fn noise_is_smooth_and_in_range() {
        let mut prev = noise(0.0, 1);
        for k in 1..2000 {
            let v = noise(k as f64 * 0.01, 1);
            assert!((0.0..=1.0).contains(&v));
            assert!((v - prev).abs() < 0.1);
            prev = v;
        }
    }

    #[test]
    fn draws_at_any_size() {
        for (c, r) in [(0, 0), (1, 1), (20, 5), (39, 30), (60, 8), (80, 24), (109, 60), (110, 12), (250, 70)]
        {
            let mut d = Dashboard::default();
            let mut out = Vec::new();
            for n in 0..90u64 {
                out.clear();
                let size = Size { cols: c, rows: r, px_w: 0, px_h: 0 };
                d.frame(&mut Frame { size, t: n as f64 / 30.0, dt: 1.0 / 30.0, out: &mut out });
            }
            assert!(c == 0 || r == 0 || !out.is_empty());
        }
    }
}

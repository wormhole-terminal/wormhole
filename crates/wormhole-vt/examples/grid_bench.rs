//! The grid's speed and memory.
//!
//! ```sh
//! cargo run --release -p wormhole-vt --example grid_bench
//! cargo run --release -p wormhole-vt --example grid_bench -- write scroll   # only these
//! ```
//!
//! Sections: `write` (ASCII runs, MB/s), `scroll` (lines per second, a full screen and regions),
//! `snapshot` (the copy of a 200x60 screen under the parser's lock, µs), `reflow` (10,000 lines of
//! history to a new width), `clusters` (cells with combining marks, emoji and ZWJ sequences, with
//! the allocator calls each costs), `links` (hyperlinked text), `memory` (bytes per history row,
//! counted at the allocator). Each timing is the best of several runs.
#![allow(clippy::print_stdout)] // a command line tool: the table is the output

use std::hint::black_box;
use std::time::{Duration, Instant};

use wormhole_vt::ViewRow;
use wormhole_vt::grid::{Grid, Pen, Region, Style};
use wormhole_vt::{Color, Column, Flags, Hyperlink, Line, NamedColor, Point};

#[path = "../tests/common/mod.rs"]
mod common;
use common::alloc::{Counting, allocations, live_bytes};
use common::rng::Rng;

#[global_allocator]
static ALLOCATOR: Counting = Counting;

const BG: Color = Color::Named(NamedColor::Background);

/// `GRID_BENCH_RUNS=n` multiplies the runs each timing takes the best of: on a busy machine a longer
/// search for an undisturbed run is what makes the numbers repeat.
fn best<F: FnMut() -> R, R>(runs: usize, mut f: F) -> Duration {
    let factor = std::env::var("GRID_BENCH_RUNS").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    (0..runs * factor)
        .map(|_| {
            let start = Instant::now();
            black_box(f());
            start.elapsed()
        })
        .min()
        .unwrap_or_default()
}

fn plain() -> Pen {
    Pen::default()
}

fn colored(n: u8) -> Pen {
    Pen { style: Style { fg: Color::Indexed(n), ..Style::DEFAULT }, ..Pen::default() }
}

/// Text of `len` bytes, deterministic, printable ASCII.
fn text(len: usize, seed: usize) -> Vec<u8> {
    (0..len).map(|i| b' ' + ((i * 7 + seed * 13) % 95) as u8).collect()
}

// ---- write ----

fn bench_write() {
    println!("== write: printable ASCII into a 200x60 grid (input MB/s; 1 MB = 10^6 bytes)");
    let (cols, rows) = (200, 60);
    let line = text(cols, 1);
    let short = text(40, 2);
    let rounds = 2000;

    let run = |name: &str, pens: &[Pen], runs_of: &[u8], prefill: Option<&Pen>| {
        let mut grid = Grid::new(cols, rows, 1 << 20);
        if let Some(pen) = prefill {
            for y in 0..rows {
                grid.write_ascii(y, 0, &line, pen);
            }
        }
        let mut bytes = 0usize;
        let d = best(7, || {
            bytes = 0;
            for round in 0..rounds {
                for y in 0..rows {
                    let pen = &pens[(round + y) % pens.len()];
                    bytes += grid.write_ascii(y, 0, runs_of, pen);
                }
            }
        });
        println!(
            "  {name:<46} {:>8.0} MB/s  {:>6.1} ns per call",
            bytes as f64 / d.as_secs_f64() / 1e6,
            d.as_nanos() as f64 / (rounds * rows) as f64
        );
    };
    run("200-byte rows, default style, blank cells", &[plain()], &line, None);
    run("200-byte rows, default style over default", &[plain()], &line, Some(&plain()));
    run("200-byte rows, one color over blank", &[colored(2)], &line, None);
    run(
        "200-byte rows, 4 colors alternating per row",
        &[colored(1), colored(2), colored(3), colored(4)],
        &line,
        None,
    );
    run("200-byte rows, one color over the same color", &[colored(2)], &line, Some(&colored(2)));
    run("200-byte rows, default over colored (releases)", &[plain()], &line, Some(&colored(5)));
    run("40-byte runs, default style", &[plain()], &short, None);
    run("40-byte runs, 4 colors", &[colored(1), colored(2), colored(3), colored(4)], &short, None);
}

// ---- scroll ----

fn filled(cols: usize, rows: usize, limit: usize) -> Grid {
    let mut grid = Grid::new(cols, rows, limit);
    for y in 0..rows {
        grid.write_ascii(y, 0, &text(cols.min(60), y), &colored((y % 6) as u8));
    }
    grid
}

fn bench_scroll() {
    println!("== scroll: lines per second");
    let line = text(80, 3);
    for (cols, rows) in [(80, 24), (200, 60)] {
        let region = Region::full(cols, rows);
        let mut grid = filled(cols, rows, 10 << 20);
        let n = 2_000_000;
        let d = best(5, || {
            for _ in 0..n {
                grid.scroll_up(region, 1, BG);
            }
        });
        println!(
            "  {cols}x{rows} full screen, history on, scroll only       {:>7.1} M lines/s  {:>6.1} ns",
            n as f64 / d.as_secs_f64() / 1e6,
            d.as_nanos() as f64 / n as f64
        );
        let mut grid = filled(cols, rows, 10 << 20);
        let d = best(5, || {
            for i in 0..n {
                grid.write_ascii(rows - 1, 0, &line[..cols.min(80)], &colored((i % 5) as u8));
                grid.scroll_up(region, 1, BG);
            }
        });
        println!(
            "  {cols}x{rows} full screen, history on, write a line + scroll {:>5.1} M lines/s  {:>6.1} ns",
            n as f64 / d.as_secs_f64() / 1e6,
            d.as_nanos() as f64 / n as f64
        );
        let mut grid = filled(cols, rows, 0);
        let d = best(5, || {
            for _ in 0..n {
                grid.scroll_up(region, 1, BG);
            }
        });
        println!(
            "  {cols}x{rows} full screen, no history (alt screen)         {:>7.1} M lines/s  {:>6.1} ns",
            n as f64 / d.as_secs_f64() / 1e6,
            d.as_nanos() as f64 / n as f64
        );
        let part = Region { top: 1, bottom: rows - 2, left: 0, right: cols - 1 };
        let mut grid = filled(cols, rows, 0);
        let d = best(5, || {
            for _ in 0..n {
                grid.scroll_up(part, 1, BG);
            }
        });
        println!(
            "  {cols}x{rows} region rows 1..{}, one page                  {:>7.1} M lines/s  {:>6.1} ns",
            rows - 2,
            n as f64 / d.as_secs_f64() / 1e6,
            d.as_nanos() as f64 / n as f64
        );
        let mut grid = Grid::with_page_rows(cols, rows, 10 << 20, rows / 2 + 1);
        for y in 0..rows {
            grid.write_ascii(y, 0, &text(cols.min(60), y), &colored((y % 6) as u8));
        }
        let m = 200_000;
        let d = best(5, || {
            for _ in 0..m {
                grid.scroll_up(part, 1, BG);
            }
        });
        println!(
            "  {cols}x{rows} region rows 1..{}, across two pages (copies) {:>7.2} M lines/s  {:>6.0} ns",
            rows - 2,
            m as f64 / d.as_secs_f64() / 1e6,
            d.as_nanos() as f64 / m as f64
        );
        let margins = Region { top: 0, bottom: rows - 1, left: 2, right: cols - 3 };
        let mut grid = filled(cols, rows, 0);
        let d = best(5, || {
            for _ in 0..m {
                grid.scroll_up(margins, 1, BG);
            }
        });
        println!(
            "  {cols}x{rows} left/right margins (DECLRMM)                 {:>7.2} M lines/s  {:>6.0} ns",
            m as f64 / d.as_secs_f64() / 1e6,
            d.as_nanos() as f64 / m as f64
        );
        let mut grid = filled(cols, rows, 0);
        let d = best(5, || {
            for _ in 0..m {
                grid.insert_lines(rows / 2, Region::full(cols, rows), 1, BG);
                grid.delete_lines(rows / 2, Region::full(cols, rows), 1, BG);
            }
        });
        println!(
            "  {cols}x{rows} insert + delete one line mid-screen          {:>7.2} M pairs/s  {:>6.0} ns",
            m as f64 / d.as_secs_f64() / 1e6,
            d.as_nanos() as f64 / m as f64
        );
    }
}

// ---- snapshot ----

fn colored_text_screen(cols: usize, rows: usize) -> Grid {
    let mut grid = Grid::new(cols, rows, 1 << 20);
    let blue_bold = Pen {
        style: Style { fg: Color::Indexed(4), attrs: Flags::BOLD, ..Style::DEFAULT },
        ..Pen::default()
    };
    for y in 0..rows {
        grid.write_ascii(y, 0, b"* ", &blue_bold);
        grid.write_ascii(
            y,
            2,
            format!("line {y}").as_bytes(),
            &Pen {
                style: Style {
                    fg: Color::Rgb(wormhole_vt::Rgb { r: (y * 7) as u8, g: 90, b: 200 }),
                    ..Style::DEFAULT
                },
                ..Pen::default()
            },
        );
        grid.write_ascii(y, 12, &text(cols - 14, y), &plain());
    }
    grid
}

fn blocks_screen(cols: usize, rows: usize) -> Grid {
    let mut grid = Grid::new(cols, rows, 1 << 20);
    for y in 0..rows {
        for x in 0..cols {
            let (r, g, b) =
                (((y * 7 + x) % 256) as u8, ((x * 13) % 256) as u8, ((y * 29 + x * 3) % 256) as u8);
            let style = Style {
                fg: Color::Rgb(wormhole_vt::Rgb { r, g, b }),
                bg: Color::Rgb(wormhole_vt::Rgb { r: b, g: r, b: g }),
                ..Style::DEFAULT
            };
            grid.write_cluster(y, x, '\u{2580}', &[], 1, &Pen { style, ..Pen::default() });
        }
    }
    grid
}

fn marked_screen(cols: usize, rows: usize) -> Grid {
    let mut grid = Grid::new(cols, rows, 1 << 20);
    for y in 0..rows {
        let link = Hyperlink { id: format!("id{}", y / 10).into(), uri: "https://example.com/file".into() };
        grid.write_ascii(
            y,
            0,
            &text(cols, y),
            &Pen {
                link: Some(link),
                content: wormhole_vt::Content::Output,
                block: 1 + (y / 10) as u32,
                ..plain()
            },
        );
    }
    grid
}

fn bench_snapshot() {
    println!("== snapshot: copy of a 200x60 screen into the types the app draws (µs per copy)");
    let (cols, rows) = (200, 60);
    for (name, mut grid) in [
        ("colored text", colored_text_screen(cols, rows)),
        ("true color blocks (every cell unique)", blocks_screen(cols, rows)),
        ("output with hyperlinks and block marks", marked_screen(cols, rows)),
        ("blank screen", Grid::new(cols, rows, 1 << 20)),
    ] {
        let mut out: Vec<ViewRow> = Vec::new();
        grid.copy_viewport(&mut out, true);
        let n = 2_000;
        let full = best(30, || {
            for _ in 0..n {
                grid.copy_viewport(&mut out, true);
            }
        });
        let dirty = best(30, || {
            for i in 0..n {
                for k in 0..3 {
                    // Touch a row the way output does: the same text again.
                    grid.write_ascii((i + k * 20) % rows, 0, b"x", &plain());
                }
                grid.copy_viewport(&mut out, false);
            }
        });
        println!(
            "  {name:<42} full {:>6.2} µs   3 dirty rows {:>6.2} µs",
            full.as_secs_f64() * 1e6 / n as f64,
            dirty.as_secs_f64() * 1e6 / n as f64
        );
    }
}

// ---- reflow ----

fn history(cols: usize, lines: usize) -> Grid {
    // A mix like real output: mostly short lines, some long ones that wrap, some colored.
    let mut grid = Grid::new(cols, 40, 1 << 30);
    let mut rng = Rng::new(12345);
    let mut next = move |n: usize| rng.below(n);
    let region = Region::full(cols, 40);
    let bottom = 39;
    for i in 0..lines {
        let long = next(10) < 3;
        let len = if long { next(250) + cols } else { next(cols - 10) + 5 };
        let pen = if i % 3 == 0 { colored((i % 7) as u8) } else { plain() };
        let bytes = text(len, i);
        let mut x = 0;
        while x < bytes.len() {
            let chunk = (bytes.len() - x).min(cols);
            grid.write_ascii(bottom, 0, &bytes[x..x + chunk], &pen);
            x += chunk;
            if x < bytes.len() {
                grid.set_wrapped(bottom, true);
                grid.scroll_up(region, 1, BG);
            }
        }
        grid.scroll_up(region, 1, BG);
    }
    grid
}

fn bench_reflow() {
    println!("== reflow: 10,000 lines of history (30% longer than the width, a third colored)");
    for (from, to) in [(80, 120), (80, 50), (120, 80), (200, 100)] {
        let proto = history(from, 10_000);
        let rows_before = proto.history_len();
        let mut times = Vec::new();
        let mut rows_after = 0;
        for _ in 0..5 {
            let mut grid = history(from, 10_000);
            let start = Instant::now();
            grid.resize(to, 40, Point::new(Line(39), Column(0)), true);
            times.push(start.elapsed());
            rows_after = grid.history_len();
        }
        let d = times.into_iter().min().unwrap_or_default();
        println!(
            "  {from:>3} -> {to:>3} columns: {:>7.2} ms   ({rows_before} -> {rows_after} history rows)",
            d.as_secs_f64() * 1e3
        );
    }
}

// ---- clusters ----

/// Clusters as the terminal writes them: a base and the code points that follow it in its cell.
const CLUSTERS: &[(&str, char, &[char], u8)] = &[
    ("e + combining acute (1 mark)", 'e', &['\u{301}'], 1),
    ("emoji + skin tone (wide, 1 mark)", '\u{1F44D}', &['\u{1F3FD}'], 2),
    ("flag (wide, 1 mark)", '\u{1F1E9}', &['\u{1F1EA}'], 2),
    (
        "ZWJ family (wide, 6 marks)",
        '\u{1F468}',
        &['\u{200D}', '\u{1F469}', '\u{200D}', '\u{1F467}', '\u{200D}', '\u{1F466}'],
        2,
    ),
];

fn bench_clusters() {
    println!("== clusters: write_cluster with marks into a 200x60 grid (ns and allocator calls per cluster)");
    let (cols, rows) = (200, 60);
    let pen = colored(3);
    for &(name, c, marks, width) in CLUSTERS {
        let per_row = cols / width as usize;
        let mut grid = Grid::new(cols, rows, 1 << 20);
        let mut calls = 0;
        let rounds = 400;
        let d = best(7, || {
            let before = allocations();
            for round in 0..rounds {
                for y in 0..rows {
                    for k in 0..per_row {
                        grid.write_cluster((y + round) % rows, k * width as usize, c, marks, width, &pen);
                    }
                }
            }
            calls = allocations() - before;
        });
        let n = rounds * rows * per_row;
        println!(
            "  {name:<36} {:>6.1} ns   {:>5.2} allocator calls",
            d.as_nanos() as f64 / n as f64,
            calls as f64 / n as f64
        );
    }
    // The marks arrive one at a time after their base (the terminal does this for most of them).
    let mut grid = Grid::new(cols, rows, 1 << 20);
    let mut calls = 0;
    let rounds = 400;
    let d = best(7, || {
        let before = allocations();
        for round in 0..rounds {
            for y in 0..rows {
                for k in 0..cols {
                    let (y, x) = ((y + round) % rows, k);
                    grid.write_cluster(y, x, 'a', &[], 1, &pen);
                    grid.push_mark(y, x, '\u{301}');
                    grid.push_mark(y, x, '\u{323}');
                }
            }
        }
        calls = allocations() - before;
    });
    let n = rounds * rows * cols;
    println!(
        "  {:<36} {:>6.1} ns   {:>5.2} allocator calls",
        "a + 2 marks pushed one by one",
        d.as_nanos() as f64 / n as f64,
        calls as f64 / n as f64
    );
    // Whole lines of clusters scrolling through a full history, the steady state of a terminal
    // showing emoji-heavy output.
    let region = Region::full(cols, rows);
    let mut grid = Grid::new(cols, rows, 1 << 20);
    let line = |grid: &mut Grid, i: usize| {
        let mut x = 0;
        for k in 0..60 {
            let (_, c, marks, width) = CLUSTERS[(i + k) % CLUSTERS.len()];
            grid.write_cluster(rows - 1, x, c, marks, width, &colored((k % 5) as u8));
            x += width as usize;
        }
        grid.write_ascii(rows - 1, x, &text(cols - x, i), &plain());
        grid.scroll_up(region, 1, BG);
    };
    for i in 0..20_000 {
        line(&mut grid, i);
    }
    let mut calls = 0;
    let n = 20_000;
    let d = best(5, || {
        let before = allocations();
        for i in 0..n {
            line(&mut grid, i);
        }
        calls = allocations() - before;
    });
    println!(
        "  {:<36} {:>6.0} ns per line  {:>5.2} allocator calls per line (60 clusters)",
        "emoji lines + scroll, full history",
        d.as_nanos() as f64 / n as f64,
        calls as f64 / n as f64
    );
}

// ---- links ----

fn link(n: usize) -> Hyperlink {
    Hyperlink { id: format!("id{n}").into(), uri: format!("https://example.com/file/{n}").into() }
}

fn linked(n: usize) -> Pen {
    Pen { link: Some(link(n)), ..Pen::default() }
}

fn bench_links() {
    println!(
        "== links: hyperlinked text in a 200x60 grid (ns per write_ascii call, allocator calls per call)"
    );
    let (cols, rows) = (200, 60);
    let rounds = 2000;
    let line = text(cols, 1);
    let short = text(40, 2);
    let run = |name: &str, pens: &[Pen], bytes: &[u8], prefill: Option<&Pen>| {
        let mut grid = Grid::new(cols, rows, 1 << 20);
        if let Some(pen) = prefill {
            for y in 0..rows {
                grid.write_ascii(y, 0, &line, pen);
            }
        }
        let mut calls = 0;
        let d = best(7, || {
            let before = allocations();
            for round in 0..rounds {
                for y in 0..rows {
                    grid.write_ascii(y, 0, bytes, &pens[(round + y) % pens.len()]);
                }
            }
            calls = allocations() - before;
        });
        let n = rounds * rows;
        println!(
            "  {name:<50} {:>7.1} ns   {:>5.2} calls",
            d.as_nanos() as f64 / n as f64,
            calls as f64 / n as f64
        );
    };
    let links: Vec<Pen> = (0..4).map(linked).collect();
    run("200-byte rows, no link (for scale)", &[plain()], &line, None);
    run("200-byte rows, one link over blank", &links[..1], &line, None);
    run("200-byte rows, one link over the same link", &links[..1], &line, Some(&links[0]));
    run("200-byte rows, 4 links alternating per row", &links, &line, None);
    run("200-byte rows, plain over linked (releases)", &[plain()], &line, Some(&links[0]));
    run("40-byte runs, one link", &links[..1], &short, None);
    run("40-byte runs, 4 links", &links, &short, None);

    // A link at the end of every line, then scrolled away: lines of output with a path or URL.
    let region = Region::full(cols, rows);
    let mut grid = Grid::new(cols, rows, 1 << 20);
    let output = |grid: &mut Grid, i: usize| {
        grid.write_ascii(rows - 1, 0, &line[..100], &plain());
        grid.write_ascii(rows - 1, 100, &line[100..160], &linked(i % 50));
        grid.scroll_up(region, 1, BG);
    };
    for i in 0..30_000 {
        output(&mut grid, i);
    }
    let n = 30_000;
    let mut calls = 0;
    let d = best(5, || {
        let before = allocations();
        for i in 0..n {
            output(&mut grid, i);
        }
        calls = allocations() - before;
    });
    println!(
        "  {:<50} {:>7.0} ns   {:>5.2} calls   (per line: 100 plain + 60 linked cells, scrolled)",
        "output lines with a 60-cell link + scroll",
        d.as_nanos() as f64 / n as f64,
        calls as f64 / n as f64
    );
}

// ---- memory ----

fn fill_history(cols: usize, rows: usize, style: impl Fn(usize) -> Vec<Pen>) -> (usize, usize, usize) {
    let before = live_bytes();
    let mut grid = Grid::new(cols, 24, 1 << 30);
    let region = Region::full(cols, 24);
    for i in 0..rows + 24 {
        let pens = style(i);
        // A line about 3/4 of the width, split among the pens.
        let len = cols * 3 / 4;
        let part = len / pens.len().max(1);
        let bytes = text(len, i);
        for (k, pen) in pens.iter().enumerate() {
            let from = k * part;
            let to = if k + 1 == pens.len() { len } else { from + part };
            grid.write_ascii(23, from, &bytes[from..to], pen);
        }
        grid.scroll_up(region, 1, BG);
    }
    let held = live_bytes() - before;
    (held, grid.memory_bytes(), grid.history_len())
}

fn bench_memory() {
    println!(
        "== memory: bytes per history row (allocator = live heap after, rows kept; pages = the grid's own count)"
    );
    for cols in [80, 200] {
        for (name, f) in [
            ("plain text", Box::new(|_: usize| vec![plain()]) as Box<dyn Fn(usize) -> Vec<Pen>>),
            (
                "colored (a prompt, 3 colors, like ls --color)",
                Box::new(|i: usize| vec![colored(2), colored((i % 6) as u8 + 1), plain()]),
            ),
            (
                "true color, a color per 4 cells",
                Box::new(|i: usize| {
                    (0..cols / 3 / 4)
                        .map(|k| Pen {
                            style: Style {
                                fg: Color::Rgb(wormhole_vt::Rgb {
                                    r: (i + k * 5) as u8,
                                    g: (k * 11) as u8,
                                    b: 90,
                                }),
                                ..Style::DEFAULT
                            },
                            ..Pen::default()
                        })
                        .collect()
                }),
            ),
        ] {
            let rows = 10_000;
            let (held, pages, kept) = fill_history(cols, rows, f);
            println!(
                "  {cols:>3} columns, {name:<46} {:>6.0} B/row by allocator, {:>6.0} B/row by pages   ({kept} rows)",
                held as f64 / kept as f64,
                pages as f64 / kept as f64
            );
        }
        println!("      (cells alone: {} B/row)", cols * 8);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let wanted = |name: &str| args.is_empty() || args.iter().any(|a| a == name);
    if wanted("write") {
        bench_write();
    }
    if wanted("scroll") {
        bench_scroll();
    }
    if wanted("snapshot") {
        bench_snapshot();
    }
    if wanted("reflow") {
        bench_reflow();
    }
    if wanted("clusters") {
        bench_clusters();
    }
    if wanted("links") {
        bench_links();
    }
    if wanted("memory") {
        bench_memory();
    }
}

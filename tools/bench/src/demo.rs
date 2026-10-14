//! The loop every animation runs in: one frame per tick inside a synchronized update, an overlay
//! with what the terminal is keeping up with, keys, resizes, and a showcase that plays several
//! demos in a row for a recording.

use std::io::{self, Write};
use std::time::{Duration, Instant};

use crate::canvas::{self, Rgb};
use crate::term::{self, AltScreen, Input, Key, RawMode, Size};

/// What a demo gets to draw one frame.
pub struct Frame<'a> {
    pub size: Size,
    /// Seconds of animation time (stops while paused, scaled by the speed keys).
    pub t: f64,
    /// Animation seconds since the last frame.
    pub dt: f64,
    /// Where the frame's bytes go. It starts empty; the runner adds the synchronized-update
    /// brackets and the overlay.
    pub out: &'a mut Vec<u8>,
}

pub trait Demo {
    /// Draws the whole screen. Draw every cell each frame: the screen isn't cleared in between.
    fn frame(&mut self, f: &mut Frame);
}

pub struct Entry {
    pub name: &'static str,
    pub about: &'static str,
    pub make: fn() -> Box<dyn Demo>,
}

#[derive(Clone, Debug)]
pub struct Options {
    /// Frames per second to aim for; 0 draws as fast as the terminal reads.
    pub fps: f64,
    /// Stop after this many seconds (per demo in a showcase).
    pub seconds: Option<f64>,
    /// The fps / MB/s overlay. Off unless asked for: an animation is for watching first, and `h`
    /// shows the numbers when they are the point.
    pub hud: bool,
    /// Wrap frames in DEC mode 2026 (synchronized output). Off shows what tearing looks like.
    pub sync: bool,
    /// Show each demo's name for a moment when it starts.
    pub titles: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self { fps: 60.0, seconds: None, hud: false, sync: true, titles: false }
    }
}

/// How fast frames left and how many bytes they took, over the last second or so.
struct Meter {
    since: Instant,
    frames: u32,
    bytes: usize,
    fps: f64,
    mbps: f64,
    frame_kb: f64,
}

impl Meter {
    fn new() -> Self {
        Self { since: Instant::now(), frames: 0, bytes: 0, fps: 0.0, mbps: 0.0, frame_kb: 0.0 }
    }

    fn add(&mut self, bytes: usize) {
        self.frames += 1;
        self.bytes += bytes;
        let el = self.since.elapsed().as_secs_f64();
        if el >= 0.5 {
            self.fps = self.frames as f64 / el;
            self.mbps = self.bytes as f64 / el / 1e6;
            self.frame_kb = self.bytes as f64 / self.frames as f64 / 1e3;
            *self = Self { since: Instant::now(), frames: 0, bytes: 0, ..*self };
        }
    }
}

enum Outcome {
    Quit,
    Next,
    Prev,
    TimeUp,
}

/// Plays one demo until a key or the time limit stops it.
pub fn run(entry: &Entry, opts: &Options) -> io::Result<()> {
    run_list(std::slice::from_ref(entry), opts)
}

/// Plays demos one after another (← → to move, q to stop), looping until stopped or, with a time
/// limit, once through.
pub fn run_list(entries: &[Entry], opts: &Options) -> io::Result<()> {
    let _raw = RawMode::enter()?;
    let _screen = AltScreen::enter()?;
    let mut input = Input::start();
    let mut i = 0usize;
    loop {
        let entry = &entries[i];
        match play(entry, opts, &mut input)? {
            Outcome::Quit => return Ok(()),
            Outcome::Next => i = (i + 1) % entries.len(),
            Outcome::Prev => i = (i + entries.len() - 1) % entries.len(),
            Outcome::TimeUp => {
                if i + 1 == entries.len() {
                    return Ok(());
                }
                i += 1;
            }
        }
    }
}

fn play(entry: &Entry, opts: &Options, input: &mut Input) -> io::Result<Outcome> {
    let mut demo = (entry.make)();
    let mut stdout = io::stdout().lock();
    let mut buf = Vec::with_capacity(1 << 20);
    let mut meter = Meter::new();
    let (mut t, mut speed, mut paused, mut hud) = (0.0f64, 1.0f64, false, opts.hud);
    let started = Instant::now();
    let mut last = Instant::now();
    let tick = if opts.fps > 0.0 { Some(Duration::from_secs_f64(1.0 / opts.fps)) } else { None };
    let mut next_at = Instant::now();
    let name = term::terminal_name();

    stdout.write_all(b"\x1b[0m\x1b[2J")?;
    loop {
        for key in input.keys() {
            match key {
                Key::Char('q') | Key::Esc | Key::CtrlC => return Ok(Outcome::Quit),
                Key::Right | Key::Char('n') => return Ok(Outcome::Next),
                Key::Left | Key::Char('p') => return Ok(Outcome::Prev),
                Key::Char(' ') => paused = !paused,
                Key::Char('h') => hud = !hud,
                Key::Char('+') | Key::Char('=') | Key::Up => speed = (speed * 1.25).min(8.0),
                Key::Char('-') | Key::Down => speed = (speed / 1.25).max(0.125),
                Key::Char('0') => speed = 1.0,
                _ => {}
            }
        }
        if let Some(limit) = opts.seconds
            && started.elapsed().as_secs_f64() >= limit
        {
            return Ok(Outcome::TimeUp);
        }

        let now = Instant::now();
        let real_dt = now.duration_since(last).as_secs_f64().min(0.1);
        last = now;
        let dt = if paused { 0.0 } else { real_dt * speed };
        t += dt;

        let size = term::size();
        buf.clear();
        if opts.sync {
            buf.extend_from_slice(b"\x1b[?2026h");
        }
        demo.frame(&mut Frame { size, t, dt, out: &mut buf });
        let since = started.elapsed().as_secs_f64();
        if opts.titles && since < 2.0 {
            title(&mut buf, size, entry.name, entry.about, since);
        }
        if hud {
            overlay(&mut buf, size, entry.name, &name, &meter, speed, paused, opts);
        }
        if opts.sync {
            buf.extend_from_slice(b"\x1b[?2026l");
        }
        stdout.write_all(&buf)?;
        stdout.flush()?;
        meter.add(buf.len());

        if let Some(tick) = tick {
            next_at += tick;
            let now = Instant::now();
            if next_at > now {
                std::thread::sleep(next_at - now);
            } else if now - next_at > tick * 4 {
                // Far behind (a slow terminal, a pause in the window server): don't race to catch up.
                next_at = now;
            }
        }
    }
}

/// A line along the top right: the demo, the terminal, and the frames and bytes per second the
/// terminal took from us. Took, not drew: a terminal reads output on a thread of its own and may draw
/// only some of the frames, which no program can see from outside, so the label says "sent".
#[allow(clippy::too_many_arguments)]
fn overlay(
    out: &mut Vec<u8>,
    size: Size,
    demo: &str,
    terminal: &str,
    m: &Meter,
    speed: f64,
    paused: bool,
    opts: &Options,
) {
    // Segments in the order shown, each with how soon it goes when the line doesn't fit (highest
    // first): a narrow window keeps the numbers people record it for.
    let measured = m.fps > 0.0;
    let mut parts: Vec<(u8, String)> = vec![
        (3, demo.to_string()),
        (5, terminal.to_string()),
        (4, format!("{}×{}", size.cols, size.rows)),
        // Until the first half second is measured there is nothing to show, and a zero would read as
        // a stall in a recording.
        (0, if measured { format!("{:>5.1} fps sent", m.fps) } else { "    – fps sent".to_string() }),
        (1, if measured { format!("{:>6.1} MB/s", m.mbps) } else { "     – MB/s".to_string() }),
        (2, if measured { format!("{:>4.0} KB/frame", m.frame_kb) } else { "   – KB/frame".to_string() }),
    ];
    if !opts.sync {
        parts.push((0, "no sync".to_string()));
    }
    if paused {
        parts.push((0, "paused".to_string()));
    } else if (speed - 1.0).abs() > 1e-6 {
        parts.push((0, format!("{speed:.2}×")));
    }
    let line = |parts: &[(u8, String)]| {
        format!(" {} ", parts.iter().map(|(_, p)| p.as_str()).collect::<Vec<_>>().join(" │ "))
    };
    let mut s = line(&parts);
    while s.chars().count() + 1 > size.cols as usize && parts.len() > 1 {
        let worst = parts.iter().enumerate().max_by_key(|(_, (k, _))| *k).map_or(0, |(i, _)| i);
        parts.remove(worst);
        s = line(&parts);
    }
    let width = s.chars().count() as u16;
    let col = size.cols.saturating_sub(width + 1);
    canvas::goto(out, col, 0);
    canvas::bg(out, Rgb(12, 12, 20));
    canvas::fg(out, Rgb(170, 220, 255));
    let max = size.cols.saturating_sub(col) as usize;
    out.extend_from_slice(s.chars().take(max).collect::<String>().as_bytes());
    canvas::reset(out);
}

/// The demo's name in the middle of the screen, fading out over its first two seconds.
fn title(out: &mut Vec<u8>, size: Size, name: &str, about: &str, since: f64) {
    let alpha = if since < 1.2 { 1.0 } else { 1.0 - (since - 1.2) / 0.8 };
    let heading =
        format!("  {}  ", name.to_uppercase().chars().flat_map(|c| [c, ' ']).collect::<String>().trim_end());
    let lines = [heading, format!("  {about}  ")];
    let mid = size.rows / 2;
    for (k, line) in lines.iter().enumerate() {
        let w = line.chars().count() as u16;
        let col = size.cols.saturating_sub(w) / 2;
        let row = mid.saturating_sub(1) + k as u16;
        canvas::goto(out, col, row);
        canvas::bg(out, Rgb(8, 8, 14).lerp(Rgb::BLACK, 1.0 - alpha));
        let color = if k == 0 { Rgb(255, 255, 255) } else { Rgb(150, 170, 200) };
        canvas::fg(out, Rgb::BLACK.lerp(color, alpha));
        if k == 0 {
            out.extend_from_slice(b"\x1b[1m");
        }
        out.extend_from_slice(line.as_bytes());
        canvas::reset(out);
    }
}

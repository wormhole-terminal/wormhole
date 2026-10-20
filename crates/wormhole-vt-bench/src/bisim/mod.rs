//! The repaint's bisimulation: an engine, and a replica built from its repaint, fed the same
//! bytes from then on, stay equal.
//!
//! This is the property the session host rests on (`session-host.md`, section 6): the host owns
//! the authoritative engine, the app keeps a replica, and the replica is made by parsing a
//! `Terminal::repaint` and then the program's raw bytes. If a repaint misses a piece of state the
//! two drift apart silently, so the test is exhaustive where it can be and says which piece of
//! state differs where it fails.
//!
//! A [`Scenario`] is a byte stream, the size it starts at and the resizes that happen at given
//! offsets. For a chosen set of cut points `P`:
//!
//! 1. engine A is fed the stream up to `P` (a cut inside an escape sequence or a UTF-8 character
//!    is moved to where the parser is between sequences, forward ([`Resolve::Defer`], a host that
//!    waits) or back ([`Resolve::Replay`], a host that repaints at the last such point and sends
//!    the bytes since as they came));
//! 2. A's repaint is parsed by a fresh engine B (after the seeds are applied), which must not
//!    reply to it;
//! 3. B and A are compared at once, fed the same bytes up to the next cut, and compared again:
//!    the whole state ([`StateDump`]), the parser's, the replies and events the bytes made.
//!
//! A [`Plan`] can also resize both at the cut, before or after the repaint, and a few cuts per
//! scenario run to the end of the stream instead of to the next cut ([`suffix`]).
//!
//! What a repaint says it left out ([`Gaps`]) is not hidden: a field the gap names is left out of
//! the comparison, every other field still must match, and `tests/bisimulation.rs` has a case for
//! each gap that shows the left-out field does differ, so the test is not blind to it.

mod cuts;
mod scenario;

pub use cuts::{every_byte, select};
pub use scenario::{from_case, from_corpus};

use std::fmt::Write as _;

use wormhole_vt::parser::Parser;
use wormhole_vt::terminal::repaint::{Gaps, NotReady};
use wormhole_vt::terminal::{ScreenChecksum, StateDump, Terminal, WindowOps};

use crate::engines::host_terminal;
use crate::rng::Rng;
use crate::vtt::show_bytes;

/// How a scenario starts.
#[derive(Clone, Copy, Debug)]
pub struct Setup {
    pub cols: usize,
    pub rows: usize,
    pub scrollback: usize,
    pub allow_checksum: bool,
    pub allow_window_ops: bool,
}

/// A resize at an offset of the stream: it happens after that many bytes have been parsed.
#[derive(Clone, Copy, Debug)]
pub struct Resize {
    pub at: usize,
    pub cols: usize,
    pub rows: usize,
}

/// A stream and what happens to the screen while it plays.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub name: String,
    pub setup: Setup,
    pub bytes: Vec<u8>,
    /// By `at`, ascending.
    pub resizes: Vec<Resize>,
}

/// A terminal as a host drives it: its parser, and what it says to the program and to the host.
pub struct Rig {
    pub term: Terminal,
    pub parser: Parser,
    /// The bytes the terminal sent to the program since they were last cleared.
    pub replies: Vec<u8>,
    /// The events since then, as text (a picture's bytes are summarized).
    pub events: Vec<String>,
}

impl Rig {
    /// A terminal of `cols` by `rows`, set up as `setup` says.
    pub fn new(setup: &Setup, cols: usize, rows: usize) -> Rig {
        let mut term = host_terminal(cols, rows, setup.scrollback);
        if setup.allow_checksum {
            term.set_screen_checksum(ScreenChecksum::Allow);
        }
        if setup.allow_window_ops {
            term.set_window_ops(WindowOps::Allow);
        }
        Rig { term, parser: Parser::new(), replies: Vec::new(), events: Vec::new() }
    }

    /// Parse `bytes`, keeping what the terminal said.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.term.advance(&mut self.parser, bytes);
        self.replies.extend_from_slice(self.term.replies());
        self.term.clear_replies();
        for event in self.term.drain_events() {
            self.events.push(event_summary(&format!("{event:?}")));
        }
    }

    /// Forget the replies and events so far.
    pub fn forget(&mut self) {
        self.replies.clear();
        self.events.clear();
    }

    /// Play `scenario` from byte `from` to byte `to`, applying the resizes at offsets in
    /// `from..=to` (the ones at `from` itself only when `from` is 0: a rig that arrived at `from`
    /// has had them).
    pub fn play(&mut self, scenario: &Scenario, from: usize, to: usize) {
        let mut pos = from;
        for resize in &scenario.resizes {
            if resize.at < from || (resize.at == from && from != 0) {
                continue;
            }
            if resize.at > to {
                break;
            }
            self.feed(&scenario.bytes[pos..resize.at]);
            pos = resize.at;
            self.term.resize(resize.cols, resize.rows);
        }
        self.feed(&scenario.bytes[pos..to]);
    }

    pub fn state(&self) -> StateDump {
        self.term.dump_state()
    }

    pub fn size(&self) -> (usize, usize) {
        (self.term.columns(), self.term.screen_lines())
    }
}

/// An event as text, cut short: a picture's payload can be megabytes.
fn event_summary(text: &str) -> String {
    const MAX: usize = 160;
    if text.len() <= MAX {
        return text.to_string();
    }
    let mut end = MAX;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… ({} bytes, {:x})", &text[..end], text.len(), crate::corpus::fnv1a(text.as_bytes()))
}

/// Where a cut inside a sequence goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolve {
    /// To the next point between sequences: the host waits for the sequence to end.
    Defer,
    /// To the last such point before it, the replica being sent the bytes since the repaint as
    /// they came.
    Replay,
}

/// What happens to the size at the cut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeAtCut {
    None,
    /// The repaint is made at the old size and both then resize.
    After,
    /// The engine resizes first and the repaint is made at the new size.
    Before,
}

/// What to check.
#[derive(Clone, Debug)]
pub struct Plan {
    /// Cut points, in any order.
    pub cuts: Vec<usize>,
    pub resolve: Resolve,
    pub resize: ResizeAtCut,
    /// How far past the last cut the last window runs.
    pub tail: usize,
    /// Seeds the sizes the cuts resize to.
    pub seed: u64,
}

impl Plan {
    pub fn new(cuts: Vec<usize>, resolve: Resolve) -> Plan {
        Plan { cuts, resolve, resize: ResizeAtCut::None, tail: 4096, seed: 1 }
    }
}

/// What a check covered.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    /// Repaints made and compared.
    pub repaints: usize,
    /// Cuts that were inside a sequence and were moved.
    pub moved: usize,
    /// The bytes of all the repaints.
    pub repaint_bytes: usize,
    /// Every gap any repaint reported.
    pub gaps: Gaps,
    /// Repaints with a gap that steers later bytes ([`Gaps::STICKY`]): compared at the cut only.
    pub sticky: usize,
}

impl Stats {
    pub fn add(&mut self, other: Stats) {
        self.repaints += other.repaints;
        self.sticky += other.sticky;
        self.moved += other.moved;
        self.repaint_bytes += other.repaint_bytes;
        self.gaps |= other.gaps;
    }
}

/// A divergence: where, and what differs.
#[derive(Debug)]
pub struct Failure {
    pub scenario: String,
    /// The cut as asked for, and as it was resolved.
    pub cut: usize,
    pub at: usize,
    pub how: String,
    pub gaps: Gaps,
    pub what: String,
    /// The bytes around the cut.
    pub context: String,
}

impl Failure {
    /// The first thing that differed, with indexes left out (`primary/row[*]`): what a list of
    /// failures is grouped by.
    pub fn kind(&self) -> String {
        let first = match self.what.split("(right):\n").nth(1) {
            Some(rest) => rest.lines().next().unwrap_or("").to_string(),
            None => self.what.lines().find(|l| !l.starts_with(' ')).unwrap_or("").to_string(),
        };
        let mut out = String::new();
        let mut inside = false;
        for c in first.chars() {
            match c {
                '[' => {
                    inside = true;
                    out.push_str("[*");
                }
                ']' => {
                    inside = false;
                    out.push(']');
                }
                _ if inside => {}
                _ => out.push(c),
            }
        }
        out
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}: cut {} (repainted at {}, {}) diverged", self.scenario, self.cut, self.at, self.how)?;
        writeln!(f, "gaps the repaint reported: {:?}", self.gaps)?;
        writeln!(f, "around the cut: {}", self.context)?;
        write!(f, "{}", self.what)
    }
}

/// Which fields a gap leaves out of the comparison.
fn masked(name: &str, gaps: Gaps) -> bool {
    (gaps.contains(Gaps::PLACEMENTS) && name.contains("/placement["))
        || (gaps.contains(Gaps::CLUSTER) && name == "a/cluster")
        || (gaps.intersects(Gaps::TOUCH | Gaps::LAST_CHAR) && name == "a/last_char")
        || (gaps.contains(Gaps::HISTORY_CLEARED) && name == "a/history_just_cleared")
        || (gaps.contains(Gaps::LOW_LEVEL)
            && (name == "a/charsets" || name == "a/cursor" || name.starts_with("a/saved[")))
        || (gaps.contains(Gaps::IMAGES)
            && (name == "a/graphics/tables" || name == "a/graphics/gate" || name == "a/graphics/transfer"))
}

fn without_gaps(dump: &StateDump, gaps: Gaps) -> StateDump {
    let mut fields: Vec<(String, String)> =
        dump.fields.iter().filter(|(name, _)| !masked(name, gaps)).cloned().collect();
    if gaps.contains(Gaps::WRAP) {
        // The last row of a screen that wraps has no row below it to wrap into.
        for prefix in ["primary/row[", "alternate/row["] {
            if let Some((_, value)) = fields.iter_mut().rev().find(|(name, _)| name.starts_with(prefix)) {
                *value = value.replacen(" wrap ", " ", 1);
            }
        }
        // And a tab mark in the last column of a wrapped row, which only a reflow makes.
        for (name, value) in &mut fields {
            if name.contains("/row[") && value.contains(" f200") {
                *value = value.replace(" f200", "");
            }
        }
    }
    StateDump { fields }
}

/// What differs between two rigs' states, `None` when nothing does.
fn differ(a: &Rig, b: &Rig, gaps: Gaps) -> Option<String> {
    let (da, db) = (without_gaps(&a.state(), gaps), without_gaps(&b.state(), gaps));
    let mut what = String::new();
    if !da.same(&db) {
        let _ = write!(what, "state of the engine (left) and the replica (right):\n{}", da.diff(&db, 8));
    }
    // The parser's own state: which one it is in and the bytes of a character it holds. The
    // parameters of the last sequence are leftovers the next sequence overwrites.
    let essence = |p: &Parser| {
        let text = format!("{p:?}");
        let state = text.split(", cur").next().unwrap_or("").to_string();
        let held = text.find("pending:").map_or("", |at| &text[at..]).to_string();
        format!("{state} {held}")
    };
    let (pa, pb) = (essence(&a.parser), essence(&b.parser));
    if pa != pb {
        let _ = write!(what, "parser:\n    left : {pa}\n    right: {pb}\n");
    }
    (!what.is_empty()).then_some(what)
}

/// What the bytes of a window made the two say, when they say different things.
fn say_differ(a: &Rig, b: &Rig, gaps: Gaps) -> Option<String> {
    // A picture's answer waits for the host's decode, and the replica never saw the pictures
    // the repaint left out: what they hold back is different.
    if gaps.intersects(Gaps::PLACEMENTS | Gaps::IMAGES) {
        return None;
    }
    let mut what = String::new();
    if a.replies != b.replies {
        let _ = writeln!(
            what,
            "replies:\n    engine : {}\n    replica: {}",
            show_bytes(&a.replies),
            show_bytes(&b.replies)
        );
    }
    if a.events != b.events {
        let first = a.events.iter().zip(&b.events).position(|(x, y)| x != y).unwrap_or(0);
        let _ = writeln!(
            what,
            "events (first difference at {first}):\n    engine : {:?}\n    replica: {:?}",
            a.events.get(first),
            b.events.get(first)
        );
    }
    (!what.is_empty()).then_some(what)
}

/// Which offsets are between sequences, for the scenario played from its start.
fn boundaries(scenario: &Scenario) -> Vec<bool> {
    let mut rig = Rig::new(&scenario.setup, scenario.setup.cols, scenario.setup.rows);
    let n = scenario.bytes.len();
    let mut ready = Vec::with_capacity(n + 1);
    let mut next_resize = 0;
    for at in 0..=n {
        while let Some(r) = scenario.resizes.get(next_resize).filter(|r| r.at == at) {
            rig.term.resize(r.cols, r.rows);
            next_resize += 1;
        }
        ready.push(rig.term.repaint_ready(&rig.parser).is_ok());
        if at < n {
            rig.term.advance(&mut rig.parser, &scenario.bytes[at..=at]);
            rig.term.clear_replies();
            rig.term.drain_events().for_each(drop);
        }
    }
    ready
}

fn resolve(ready: &[bool], cut: usize, how: Resolve) -> Option<usize> {
    let cut = cut.min(ready.len() - 1);
    match how {
        Resolve::Defer => (cut..ready.len()).find(|&i| ready[i]),
        Resolve::Replay => (0..=cut).rev().find(|&i| ready[i]),
    }
}

/// The bytes of the stream around `at`, as text.
fn context(scenario: &Scenario, at: usize) -> String {
    let (lo, hi) = (at.saturating_sub(24), (at + 24).min(scenario.bytes.len()));
    format!("{} | {}", show_bytes(&scenario.bytes[lo..at]), show_bytes(&scenario.bytes[at..hi]))
}

fn fail(scenario: &Scenario, cut: usize, at: usize, how: String, gaps: Gaps, what: String) -> Box<Failure> {
    Box::new(Failure {
        scenario: scenario.name.clone(),
        cut,
        at,
        how,
        gaps,
        what,
        context: context(scenario, at),
    })
}

/// The replica for `a`: a fresh terminal of `a`'s size that has parsed `a`'s repaint.
fn replica(
    scenario: &Scenario,
    a: &Rig,
    bytes: &mut Vec<u8>,
) -> Result<(Rig, wormhole_vt::terminal::repaint::Repainted), NotReady> {
    bytes.clear();
    let done = a.term.repaint(&a.parser, bytes)?;
    let (cols, rows) = a.size();
    let mut b = Rig::new(&scenario.setup, cols, rows);
    b.term.seed(&done.seeds);
    b.feed(bytes);
    Ok((b, done))
}

/// Check the plan's cuts, window by window; see the module docs.
pub fn check(scenario: &Scenario, plan: &Plan) -> Result<Stats, Box<Failure>> {
    let ready = boundaries(scenario);
    let n = scenario.bytes.len();
    let mut stats = Stats::default();
    let mut points: Vec<(usize, usize)> =
        plan.cuts.iter().filter_map(|&cut| resolve(&ready, cut, plan.resolve).map(|at| (cut, at))).collect();
    points.sort_by_key(|&(_, at)| at);
    points.dedup_by_key(|&mut (_, at)| at);
    stats.moved = points.iter().filter(|&&(cut, at)| cut != at).count();
    let mut a = Rig::new(&scenario.setup, scenario.setup.cols, scenario.setup.rows);
    a.play(scenario, 0, 0);
    let mut pos = 0;
    let mut rng = Rng::new(plan.seed);
    let mut bytes = Vec::new();
    for (i, &(cut, at)) in points.iter().enumerate() {
        a.play(scenario, pos, at);
        // The new size, if the cut resizes.
        let (cols, rows) = (
            rng.range(scenario.setup.cols.saturating_sub(7).max(2), scenario.setup.cols + 9),
            // Two rows at least: a screen of one row cannot go back to the row above to take back
            // the blank a wrapped line was printed to (`Gaps::WRAP`), and nobody works in one.
            rng.range(scenario.setup.rows.saturating_sub(5).max(2), scenario.setup.rows + 6),
        );
        let how = if plan.resize == ResizeAtCut::None {
            format!("{:?}", plan.resolve)
        } else {
            format!("{:?}, resize {:?} from {:?} to {cols}x{rows}", plan.resolve, plan.resize, a.size())
        };
        let fail_here = |gaps: Gaps, what: String| fail(scenario, cut, at, how.clone(), gaps, what);
        if plan.resize == ResizeAtCut::Before {
            a.term.resize(cols, rows);
        }
        let (mut b, done) = match replica(scenario, &a, &mut bytes) {
            Ok(made) => made,
            Err(_) => {
                return Err(fail_here(
                    Gaps::empty(),
                    "the engine was not ready at a point the scan said it was".into(),
                ));
            }
        };
        stats.repaints += 1;
        stats.repaint_bytes += bytes.len();
        stats.gaps |= done.gaps;
        if !b.replies.is_empty() {
            return Err(fail_here(
                done.gaps,
                format!(
                    "the repaint made the replica reply: {}\nrepaint: {}",
                    show_bytes(&b.replies),
                    show_bytes(&bytes)
                ),
            ));
        }
        if let Some(what) = differ(&a, &b, done.gaps) {
            let shown = &bytes[..bytes.len().min(1200)];
            return Err(fail_here(
                done.gaps,
                format!("right after the repaint ({} bytes: {})\n{what}", bytes.len(), show_bytes(shown)),
            ));
        }
        if plan.resize == ResizeAtCut::After {
            a.term.resize(cols, rows);
            b.term.resize(cols, rows);
            // What a gap left out steers a resize too (a history cap, a wrap pending).
            if !done.gaps.intersects(Gaps::STICKY)
                && let Some(what) = differ(&a, &b, done.gaps)
            {
                return Err(fail_here(done.gaps, format!("right after both resized\n{what}")));
            }
        }
        // The window up to the next cut.
        let end = points.get(i + 1).map_or_else(|| (at + plan.tail).min(n), |&(_, next)| next);
        a.forget();
        a.play(scenario, at, end);
        pos = end;
        if done.gaps.intersects(Gaps::STICKY) {
            // What the repaint left out steers what the bytes do: the replica is not expected to
            // follow from here, only to have been right at the cut.
            stats.sticky += 1;
            continue;
        }
        b.forget();
        b.play(scenario, at, end);
        if let Some(what) = differ(&a, &b, done.gaps) {
            return Err(fail_here(done.gaps, format!("after {} more bytes (to {end})\n{what}", end - at)));
        }
        if let Some(what) = say_differ(&a, &b, done.gaps) {
            return Err(fail_here(
                done.gaps,
                format!("in the {} bytes after the repaint (to {end})\n{what}", end - at),
            ));
        }
    }
    Ok(stats)
}

/// Check cuts to the end of the stream: B parses the whole rest and is compared with an engine that
/// parsed everything, in state and in what it said to the program.
pub fn suffix(scenario: &Scenario, cuts: &[usize], how: Resolve) -> Result<Stats, Box<Failure>> {
    let ready = boundaries(scenario);
    let n = scenario.bytes.len();
    let mut stats = Stats::default();
    let mut points: Vec<(usize, usize)> =
        cuts.iter().filter_map(|&cut| resolve(&ready, cut, how).map(|at| (cut, at))).collect();
    points.sort_by_key(|&(_, at)| at);
    points.dedup_by_key(|&mut (_, at)| at);
    let mut a = Rig::new(&scenario.setup, scenario.setup.cols, scenario.setup.rows);
    a.play(scenario, 0, 0);
    // The repaint of each cut, and how many replies the engine had made by then.
    let mut taken = Vec::new();
    let mut pos = 0;
    for &(cut, at) in &points {
        a.play(scenario, pos, at);
        pos = at;
        let mut bytes = Vec::new();
        let Ok(done) = a.term.repaint(&a.parser, &mut bytes) else { continue };
        if done.gaps.intersects(Gaps::STICKY) {
            stats.sticky += 1;
            continue;
        }
        stats.repaints += 1;
        stats.repaint_bytes += bytes.len();
        stats.gaps |= done.gaps;
        taken.push((cut, at, bytes, done, a.size(), a.replies.len()));
    }
    a.play(scenario, pos, n);
    for (cut, at, bytes, done, (cols, rows), replies) in taken {
        let mut b = Rig::new(&scenario.setup, cols, rows);
        b.term.seed(&done.seeds);
        b.feed(&bytes);
        let label = format!("{how:?}, to the end");
        if !b.replies.is_empty() {
            return Err(fail(
                scenario,
                cut,
                at,
                label,
                done.gaps,
                format!("the repaint made the replica reply: {}", show_bytes(&b.replies)),
            ));
        }
        b.forget();
        b.play(scenario, at, n);
        if let Some(what) = differ(&a, &b, done.gaps) {
            return Err(fail(
                scenario,
                cut,
                at,
                label,
                done.gaps,
                format!("at the end of the stream\n{what}"),
            ));
        }
        if !done.gaps.intersects(Gaps::PLACEMENTS | Gaps::IMAGES) && a.replies[replies..] != b.replies[..] {
            return Err(fail(
                scenario,
                cut,
                at,
                label,
                done.gaps,
                format!(
                    "replies after the cut:\n    engine : {}\n    replica: {}",
                    show_bytes(&a.replies[replies..]),
                    show_bytes(&b.replies)
                ),
            ));
        }
    }
    Ok(stats)
}

/// Run `check` for each combination of resolution and resize that makes sense for `cuts`.
pub fn check_all(scenario: &Scenario, cuts: &[usize]) -> Result<Stats, Box<Failure>> {
    let mut total = Stats::default();
    for resolve in [Resolve::Defer, Resolve::Replay] {
        for resize in [ResizeAtCut::None, ResizeAtCut::After, ResizeAtCut::Before] {
            let mut plan = Plan::new(cuts.to_vec(), resolve);
            plan.resize = resize;
            total.add(check(scenario, &plan)?);
        }
    }
    Ok(total)
}

/// What one repaint at one offset shows, for the tests that say what the gaps do.
pub struct Probe {
    pub gaps: Gaps,
    /// What differs between the engine and the replica with the gaps' fields left out, as
    /// [`check`] compares; `None` is a repaint that is right.
    pub masked: Option<String>,
    /// What differs with nothing left out: for a gap, the field it names.
    pub unmasked: Option<String>,
    pub bytes: Vec<u8>,
}

/// Play `scenario` to `at`, repaint, let `tamper` change the bytes, and compare the replica made
/// from them with the engine.
pub fn probe(scenario: &Scenario, at: usize, tamper: impl FnOnce(&mut Vec<u8>)) -> Probe {
    let mut a = Rig::new(&scenario.setup, scenario.setup.cols, scenario.setup.rows);
    a.play(scenario, 0, at);
    let mut bytes = Vec::new();
    let done = a.term.repaint(&a.parser, &mut bytes).expect("a point between sequences");
    tamper(&mut bytes);
    let (cols, rows) = a.size();
    let mut b = Rig::new(&scenario.setup, cols, rows);
    b.term.seed(&done.seeds);
    b.feed(&bytes);
    Probe {
        gaps: done.gaps,
        masked: differ(&a, &b, done.gaps),
        unmasked: differ(&a, &b, Gaps::empty()),
        bytes,
    }
}

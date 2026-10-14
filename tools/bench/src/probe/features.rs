//! The questions a terminal can be asked, and how each reply is read. The readers are plain
//! functions of one reply so they can be tested on the bytes terminals really send.

use std::io::{self, Write};

use super::model::{Decision, Outcome, Row, Section, Trace};
use super::query::{Prober, Reading};
use crate::term::{Reply, ReplyKind, Size};

/// A mode to ask about with DECRQM: number, name, and whether its current state means something
/// (it doesn't for the ones the probe itself changes before asking).
const MODES: [(u16, &str, bool); 11] = [
    (2026, "synchronized output", true),
    (2027, "grapheme clustering", true),
    (2004, "bracketed paste", true),
    (1004, "focus events", true),
    (1006, "SGR mouse", true),
    (1049, "alternate screen", false),
    (2031, "color-scheme updates", true),
    (1016, "SGR-pixels mouse", true),
    (7, "autowrap", false),
    (25, "cursor visible", false),
    (12, "cursor blink", true),
];

pub fn mode_row_name(n: u16, name: &str) -> String {
    format!("{name} ({n})")
}

/// `CSI ? n ; v $ y`: the mode's state, 0 meaning the terminal doesn't know the mode.
pub fn decrpm(mode: u16) -> impl Fn(&Reply) -> Option<u8> {
    move |r| {
        if r.kind != ReplyKind::Csi {
            return None;
        }
        let rest = r.body.strip_prefix('?')?.strip_suffix("$y")?;
        let (n, v) = rest.split_once(';')?;
        (n.parse::<u16>().ok()? == mode).then(|| v.parse().ok()).flatten()
    }
}

/// `DCS > | text ST`
pub fn xtversion(r: &Reply) -> Option<String> {
    (r.kind == ReplyKind::Dcs).then(|| r.body.strip_prefix(">|").map(str::to_string)).flatten()
}

/// `CSI > Pp ; Pv ; Pc c`, as `Pp;Pv;Pc`.
pub fn da2(r: &Reply) -> Option<String> {
    if r.kind != ReplyKind::Csi {
        return None;
    }
    r.body.strip_prefix('>')?.strip_suffix('c').map(str::to_string)
}

/// The numbers of a DA1 reply body (`?64;1;4c`).
pub fn da1_attrs(body: &str) -> Vec<u32> {
    body.trim_start_matches('?').trim_end_matches('c').split(';').filter_map(|n| n.parse().ok()).collect()
}

/// `CSI ? flags u`
pub fn kitty_keyboard(r: &Reply) -> Option<u32> {
    if r.kind != ReplyKind::Csi {
        return None;
    }
    r.body.strip_prefix('?')?.strip_suffix('u')?.parse().ok()
}

/// `APC G i=31 ; OK ST`: the message after the semicolon (`OK`, or an error naming what was wrong).
pub fn kitty_graphics(r: &Reply) -> Option<String> {
    if r.kind != ReplyKind::Apc || !r.body.starts_with('G') {
        return None;
    }
    let (head, msg) = r.body.split_once(';')?;
    head.split(',').any(|kv| kv == "i=31" || kv == "Gi=31").then(|| msg.to_string())
}

/// `OSC 1337 ; ReportCellSize = height ; width ; scale ST`, as `(width, height)` text.
pub fn iterm_cell_size(r: &Reply) -> Option<(String, String)> {
    if r.kind != ReplyKind::Osc {
        return None;
    }
    let rest = r.body.strip_prefix("1337;ReportCellSize=")?;
    let mut it = rest.split(';');
    let h = it.next()?.to_string();
    let w = it.next()?.to_string();
    Some((w, h))
}

/// A color as `rgb:RRRR/GGGG/BBBB` (one to four hex digits each), scaled to 8 bits.
pub fn parse_color(spec: &str) -> Option<(u8, u8, u8)> {
    let rest = spec.strip_prefix("rgb:")?;
    let mut parts = rest.split('/');
    let mut channel = || -> Option<u8> {
        let p = parts.next()?;
        if p.is_empty() || p.len() > 4 || !p.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let v = u32::from_str_radix(p, 16).ok()?;
        let max = (1u32 << (4 * p.len())) - 1;
        Some(((v * 255 + max / 2) / max) as u8)
    };
    let c = (channel()?, channel()?, channel()?);
    parts.next().is_none().then_some(c)
}

pub fn hex((r, g, b): (u8, u8, u8)) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// The color an `OSC <prefix> ; rgb:… ST` reply carries (`10`, `11`, or `4;1`).
pub fn osc_color(prefix: &'static str) -> impl Fn(&Reply) -> Option<(u8, u8, u8)> {
    move |r| {
        if r.kind != ReplyKind::Osc {
            return None;
        }
        parse_color(r.body.strip_prefix(prefix)?.strip_prefix(';')?)
    }
}

/// Whether a color is dark, by relative luminance.
pub fn is_dark((r, g, b): (u8, u8, u8)) -> bool {
    (0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b)) / 255.0 < 0.5
}

/// `CSI kind ; a ; b t`, as `(a, b)`.
pub fn window_report(kind: u32) -> impl Fn(&Reply) -> Option<(u32, u32)> {
    move |r| {
        if r.kind != ReplyKind::Csi {
            return None;
        }
        let mut it = r.body.strip_suffix('t')?.split(';');
        if it.next()?.parse::<u32>().ok()? != kind {
            return None;
        }
        let a = it.next()?.parse().ok()?;
        let b = it.next()?.parse().ok()?;
        it.next().is_none().then_some((a, b))
    }
}

/// `DCS 1 $ r text ST` (a valid DECRQSS reply: `(true, text)`) or `DCS 0 $ r ST` (not valid).
pub fn decrqss(r: &Reply) -> Option<(bool, String)> {
    if r.kind != ReplyKind::Dcs {
        return None;
    }
    if let Some(t) = r.body.strip_prefix("1$r") {
        return Some((true, t.to_string()));
    }
    r.body.strip_prefix("0$r").map(|t| (false, t.to_string()))
}

/// Whether a DECRQSS reply for SGR carries the color `10;20;30` we set (`:` and `;` forms alike).
pub fn sgr_has_test_color(text: &str) -> bool {
    text.replace(':', ";").contains("10;20;30")
}

/// The words for a DECSCUSR value.
pub fn cursor_style_name(n: u32) -> &'static str {
    match n {
        0 | 1 => "blinking block",
        2 => "steady block",
        3 => "blinking underline",
        4 => "steady underline",
        5 => "blinking bar",
        6 => "steady bar",
        _ => "unknown style",
    }
}

fn cursor_style(text: &str) -> Option<&'static str> {
    text.strip_suffix(" q")?.parse().ok().map(cursor_style_name)
}

/// What a row says when the terminal had no value for it.
fn absent(d: Decision) -> (Outcome, &'static str) {
    match d {
        Decision::Sentinel => (Outcome::No, "not reported"),
        Decision::Unparsed => (Outcome::Unknown, "odd reply"),
        Decision::Skipped => (Outcome::Unknown, "not asked"),
        _ => (Outcome::Unknown, "no reply"),
    }
}

fn row(section: Section, name: &str, how: &str, outcome: Outcome, note: &str, trace: Trace) -> Row {
    Row { section, name: name.to_string(), outcome, note: note.to_string(), how: how.to_string(), trace }
}

/// A row from a reading: `found` turns the value into the outcome and note; without one the row
/// says why not.
fn from_reading<T>(
    section: Section,
    name: &str,
    how: &str,
    rd: Reading<T>,
    found: impl FnOnce(T) -> (Outcome, String),
) -> Row {
    match rd.value {
        Some(v) => {
            let (outcome, note) = found(v);
            row(section, name, how, outcome, &note, rd.trace)
        }
        None => {
            let (outcome, note) = absent(rd.trace.decided);
            row(section, name, how, outcome, note, rd.trace)
        }
    }
}

fn env_row(name: &str, var: &str, extra: Option<&str>) -> Row {
    let value = std::env::var(var).ok().filter(|v| !v.is_empty());
    let shown = match (&value, extra.and_then(|e| std::env::var(e).ok()).filter(|v| !v.is_empty())) {
        (Some(v), Some(x)) => format!("{v} {x}"),
        (Some(v), None) => v.clone(),
        (None, _) => "not set".to_string(),
    };
    row(
        Section::Identity,
        name,
        &format!("${var}"),
        Outcome::Value(shown),
        "",
        Trace::without_query(Decision::Environment),
    )
}

/// The multiplexer the terminal is behind, if the environment says so: it answers some queries
/// itself and swallows others.
fn multiplexer() -> &'static str {
    let set = |v: &str| std::env::var(v).is_ok_and(|x| !x.is_empty());
    if set("TMUX") {
        "tmux"
    } else if set("ZELLIJ") {
        "zellij"
    } else if set("STY") {
        "screen"
    } else {
        "none"
    }
}

pub struct Probed {
    pub rows: Vec<Row>,
    /// DECRPM's value for mode 2027, when the terminal answered about it.
    pub mode_2027: Option<u8>,
}

pub fn probe<W: Write>(p: &mut Prober<'_, W>, size: Size) -> io::Result<Probed> {
    let mut rows = Vec::new();

    let da1 = p.da1()?;
    let attrs = da1.value.as_deref().map(da1_attrs);
    let da1_trace = da1.trace.clone();
    let da1_row = from_reading(Section::Identity, "device attributes (DA1)", "CSI c", da1, |b| {
        (
            Outcome::Value(da1_attrs(&b).iter().map(u32::to_string).collect::<Vec<_>>().join(";")),
            String::new(),
        )
    });

    rows.push(from_reading(
        Section::Identity,
        "terminal (XTVERSION)",
        "CSI > 0 q",
        p.ask(b"\x1b[>0q", xtversion)?,
        |t| (Outcome::Value(t), String::new()),
    ));
    rows.push(da1_row);
    rows.push(from_reading(
        Section::Identity,
        "secondary attributes (DA2)",
        "CSI > c",
        p.ask(b"\x1b[>c", da2)?,
        |t| (Outcome::Value(t), String::new()),
    ));
    rows.push(env_row("TERM_PROGRAM", "TERM_PROGRAM", Some("TERM_PROGRAM_VERSION")));
    rows.push(env_row("TERM", "TERM", None));
    rows.push(env_row("COLORTERM", "COLORTERM", None));
    rows.push(row(
        Section::Identity,
        "multiplexer",
        "$TMUX, $STY, $ZELLIJ",
        Outcome::Value(multiplexer().to_string()),
        "",
        Trace::without_query(Decision::Environment),
    ));

    let mut mode_2027 = None;
    for (n, what, show_state) in MODES {
        let rd = p.ask(format!("\x1b[?{n}$p").as_bytes(), decrpm(n))?;
        if n == 2027 {
            mode_2027 = rd.value;
        }
        rows.push(from_reading(Section::Modes, &mode_row_name(n, what), &format!("DECRQM {n}"), rd, |v| {
            let state = |s: &str| if show_state { s.to_string() } else { String::new() };
            match v {
                0 => (Outcome::No, "not recognized".to_string()),
                1 => (Outcome::Yes, state("on")),
                2 => (Outcome::Yes, state("off")),
                3 => (Outcome::Yes, "always on".to_string()),
                4 => (Outcome::No, "fixed off".to_string()),
                _ => (Outcome::Unknown, format!("odd value {v}")),
            }
        }));
    }

    rows.push(from_reading(
        Section::Modes,
        "Kitty keyboard protocol",
        "CSI ? u",
        p.ask(b"\x1b[?u", kitty_keyboard)?,
        |flags| (Outcome::Yes, format!("flags {flags}")),
    ));
    rows.push(from_reading(
        Section::Modes,
        "cursor style (DECSCUSR)",
        "DECRQSS SP q",
        p.ask(b"\x1bP$q q\x1b\\", |r| decrqss(r).and_then(|(ok, t)| ok.then(|| cursor_style(&t)).flatten()))?,
        |name| (Outcome::Value(name.to_string()), String::new()),
    ));

    rows.push(from_reading(
        Section::Graphics,
        "Kitty graphics",
        "APC G a=q",
        p.ask(b"\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\", kitty_graphics)?,
        |msg| (Outcome::Yes, if msg == "OK" { String::new() } else { format!("answered: {msg}") }),
    ));
    let sixel = match &attrs {
        Some(a) if a.contains(&4) => (Outcome::Yes, ""),
        Some(_) => (Outcome::No, "not in DA1"),
        None => (Outcome::Unknown, "no reply"),
    };
    rows.push(row(
        Section::Graphics,
        "Sixel",
        "DA1 attribute 4",
        sixel.0,
        sixel.1,
        Trace { micros: da1_trace.micros, ..Trace::without_query(Decision::Derived) },
    ));
    rows.push(from_reading(
        Section::Graphics,
        "iTerm2 inline images",
        "OSC 1337 ReportCellSize",
        p.ask(b"\x1b]1337;ReportCellSize\x1b\\", iterm_cell_size)?,
        |(w, h)| (Outcome::Yes, format!("cell {w}×{h} px")),
    ));

    rows.push(from_reading(
        Section::Colors,
        "foreground (OSC 10)",
        "OSC 10 ?",
        p.ask(b"\x1b]10;?\x1b\\", osc_color("10"))?,
        |c| (Outcome::Value(hex(c)), String::new()),
    ));
    let bg = p.ask(b"\x1b]11;?\x1b\\", osc_color("11"))?;
    let dark = match bg.value {
        Some(c) => (Outcome::Value(if is_dark(c) { "dark" } else { "light" }.to_string()), ""),
        None => (Outcome::Unknown, "no background"),
    };
    let bg_micros = bg.trace.micros;
    rows.push(from_reading(Section::Colors, "background (OSC 11)", "OSC 11 ?", bg, |c| {
        (Outcome::Value(hex(c)), String::new())
    }));
    rows.push(from_reading(
        Section::Colors,
        "palette color 1 (OSC 4)",
        "OSC 4;1;?",
        p.ask(b"\x1b]4;1;?\x1b\\", osc_color("4;1"))?,
        |c| (Outcome::Value(hex(c)), String::new()),
    ));
    rows.push(row(
        Section::Colors,
        "background brightness",
        "from OSC 11",
        dark.0,
        dark.1,
        Trace { micros: bg_micros, ..Trace::without_query(Decision::Derived) },
    ));
    rows.push(truecolor(p)?);

    rows.push(from_reading(
        Section::Sizes,
        "size in cells (CSI 18 t)",
        "CSI 18 t",
        p.ask(b"\x1b[18t", window_report(8))?,
        |(r, c)| (Outcome::Value(format!("{c}×{r}")), String::new()),
    ));
    rows.push(from_reading(
        Section::Sizes,
        "window in pixels (CSI 14 t)",
        "CSI 14 t",
        p.ask(b"\x1b[14t", window_report(4))?,
        |(h, w)| (Outcome::Value(format!("{w}×{h}")), String::new()),
    ));
    rows.push(from_reading(
        Section::Sizes,
        "cell in pixels (CSI 16 t)",
        "CSI 16 t",
        p.ask(b"\x1b[16t", window_report(6))?,
        |(h, w)| (Outcome::Value(format!("{w}×{h}")), String::new()),
    ));
    let px = if size.px_w > 0 { format!(", {}×{} px", size.px_w, size.px_h) } else { String::new() };
    rows.push(row(
        Section::Sizes,
        "size by ioctl",
        "TIOCGWINSZ",
        Outcome::Value(format!("{}×{}{px}", size.cols, size.rows)),
        "",
        Trace::without_query(Decision::Environment),
    ));

    Ok(Probed { rows, mode_2027 })
}

/// Sets a known color, asks the terminal to report its graphic rendition (DECRQSS) and looks for
/// the color in it, then restores the rendition.
fn truecolor<W: Write>(p: &mut Prober<'_, W>) -> io::Result<Row> {
    let rd = p.ask(b"\x1b[38:2::10:20:30m\x1bP$qm\x1b\\", decrqss)?;
    p.send(b"\x1b[0m")?;
    let colorterm = std::env::var("COLORTERM").unwrap_or_default();
    let hint = if colorterm.is_empty() { String::new() } else { format!("$COLORTERM={colorterm}") };
    let decided = rd.trace.decided;
    let (outcome, note) = match rd.value {
        Some((true, text)) if sgr_has_test_color(&text) => {
            (Outcome::Yes, "SGR report has the color".to_string())
        }
        Some((true, text)) => (Outcome::No, format!("SGR report lacks the color: {text}")),
        Some((false, _)) => (Outcome::Unknown, format!("DECRQSS refused {hint}").trim().to_string()),
        None if decided == Decision::Sentinel => {
            (Outcome::Unknown, format!("DECRQSS not supported {hint}").trim().to_string())
        }
        None => {
            let (o, n) = absent(decided);
            (o, format!("{n} {hint}").trim().to_string())
        }
    };
    Ok(row(Section::Colors, "truecolor (DECRQSS)", "SGR 38:2 then DECRQSS m", outcome, &note, rd.trace))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(kind: ReplyKind, body: &str) -> Reply {
        Reply { kind, body: body.to_string() }
    }

    #[test]
    fn decrpm_reads_its_own_mode_only() {
        let reply = r(ReplyKind::Csi, "?2026;2$y");
        assert_eq!(decrpm(2026)(&reply), Some(2));
        assert_eq!(decrpm(2027)(&reply), None);
        assert_eq!(decrpm(2026)(&r(ReplyKind::Csi, "?2026;0$y")), Some(0));
        assert_eq!(decrpm(2026)(&r(ReplyKind::Csi, "2026;2$y")), None);
        assert_eq!(decrpm(2026)(&r(ReplyKind::Dcs, "?2026;2$y")), None);
    }

    #[test]
    fn identity_replies() {
        assert_eq!(xtversion(&r(ReplyKind::Dcs, ">|WezTerm 20240203")).as_deref(), Some("WezTerm 20240203"));
        assert_eq!(xtversion(&r(ReplyKind::Dcs, "1$r0m")), None);
        assert_eq!(da2(&r(ReplyKind::Csi, ">1;4000;19c")).as_deref(), Some("1;4000;19"));
        assert_eq!(da2(&r(ReplyKind::Csi, "?64;1c")), None);
        assert_eq!(da1_attrs("?64;1;4;22c"), vec![64, 1, 4, 22]);
    }

    #[test]
    fn graphics_replies() {
        assert_eq!(kitty_graphics(&r(ReplyKind::Apc, "Gi=31;OK")).as_deref(), Some("OK"));
        assert_eq!(kitty_graphics(&r(ReplyKind::Apc, "Gi=31;EINVAL:bad")).as_deref(), Some("EINVAL:bad"));
        assert_eq!(kitty_graphics(&r(ReplyKind::Apc, "Gi=7;OK")), None);
        assert_eq!(
            iterm_cell_size(&r(ReplyKind::Osc, "1337;ReportCellSize=18.0;9.0;2.0")),
            Some(("9.0".to_string(), "18.0".to_string()))
        );
        assert_eq!(kitty_keyboard(&r(ReplyKind::Csi, "?31u")), Some(31));
    }

    #[test]
    fn colors_scale_to_eight_bits() {
        assert_eq!(parse_color("rgb:ffff/0000/8080"), Some((255, 0, 128)));
        assert_eq!(parse_color("rgb:ff/00/80"), Some((255, 0, 128)));
        assert_eq!(parse_color("rgb:f/0/8"), Some((255, 0, 136)));
        assert_eq!(parse_color("rgb:fffff/0/0"), None);
        assert_eq!(parse_color("rgb:ff/00"), None);
        assert_eq!(parse_color("#ffffff"), None);
        assert_eq!(osc_color("11")(&r(ReplyKind::Osc, "11;rgb:1e1e/1e1e/2e2e")), Some((30, 30, 46)));
        assert_eq!(osc_color("4;1")(&r(ReplyKind::Osc, "4;1;rgb:cc/00/00")), Some((204, 0, 0)));
        assert_eq!(osc_color("10")(&r(ReplyKind::Osc, "11;rgb:00/00/00")), None);
        assert_eq!(hex((30, 30, 46)), "#1e1e2e");
        assert!(is_dark((30, 30, 46)) && !is_dark((250, 250, 250)));
    }

    #[test]
    fn window_reports_name_their_kind() {
        assert_eq!(window_report(8)(&r(ReplyKind::Csi, "8;40;120t")), Some((40, 120)));
        assert_eq!(window_report(8)(&r(ReplyKind::Csi, "4;40;120t")), None);
        assert_eq!(window_report(4)(&r(ReplyKind::Csi, "4;800;1200t")), Some((800, 1200)));
    }

    #[test]
    fn decrqss_replies() {
        assert_eq!(
            decrqss(&r(ReplyKind::Dcs, "1$r0;38:2::10:20:30m")),
            Some((true, "0;38:2::10:20:30m".to_string()))
        );
        assert_eq!(decrqss(&r(ReplyKind::Dcs, "0$r")), Some((false, String::new())));
        assert!(sgr_has_test_color("0;38:2::10:20:30m") && sgr_has_test_color("0;38;2;10;20;30m"));
        assert!(!sgr_has_test_color("0;38;5;17m"));
        assert_eq!(cursor_style("5 q"), Some("blinking bar"));
        assert_eq!(cursor_style("2 q"), Some("steady block"));
    }
}

//! A stream without what alacritty gets wrong or leaves out.
//!
//! Each rule is a sequence (or a use of one) that alacritty does not do as the specs say, or does
//! differently from the new engine where the specs leave the choice. Rules that are bugs are
//! cases of the conformance suite (`known-fail: alacritty`); the rest say why in a line. What is
//! removed is removed from the stream both engines see.

/// What the stream so far makes some later sequences unsafe to compare.
#[derive(Default)]
struct Context {
    /// DECSTBM was set: alacritty's CUU, CUD, CPL, CNL and VPR ignore the margins.
    region: bool,
    /// A tab stop was cleared: alacritty's CBT does not go to the first column without stops.
    tabs_cleared: bool,
    /// Something since the last graphic character makes REP repeat different text in alacritty
    /// (a mark, a character set change).
    rep_unsafe: bool,
    /// The stream has a DEL somewhere, and REP never agrees afterwards.
    rep_never: bool,
    /// A locking shift is in force: alacritty's DECRC does not bring the shift back.
    shifted: bool,
    /// A locking shift was saved by DECSC (or SCOSC): the restore would bring it back.
    shift_saved: bool,
    /// A DECSTBM without a bottom margin was given: alacritty's origin mode then counts the top
    /// margin twice.
    bottomless: bool,
    /// Autowrap was switched off.
    no_wrap: bool,
    /// The alternate screen shows: asking for it again saves the cursor again in the new engine
    /// (xterm), and not in alacritty.
    alt: bool,
    /// Origin mode was set: alacritty's DECRC does not bring the mode back, and its DECSTBM
    /// without a bottom margin adds the top margin to the cursor's row twice.
    origin: bool,
    /// Text was written and the cursor has not moved since: the row may be full with a wrap due.
    /// A tab (or CHT) then goes nowhere, and the new engine keeps the wrap due where alacritty
    /// wraps first (VT510 HT: HT does not cause text to auto wrap).
    after_text: bool,
    /// The screen's height, for telling a region that is not one.
    rows: usize,
}

/// `stream` without the sequences alacritty gets wrong, and without bytes that are not UTF-8
/// (alacritty drops a lone continuation byte, the new engine prints U+FFFD, as Unicode says).
pub fn sanitize(stream: &[u8], rows: usize) -> Vec<u8> {
    // Taking something out can leave what was around it touching, as an ESC and the control that
    // followed the sequence that was taken out: until nothing more goes.
    let mut current = pass(stream, rows);
    loop {
        let next = pass(&current, rows);
        if next == current {
            return current;
        }
        current = next;
    }
}

fn pass(stream: &[u8], rows: usize) -> Vec<u8> {
    let mut valid = valid_utf8(stream);
    // DEL is ignored wherever it is (also inside a sequence); alacritty prints it as a blank
    // that REP then repeats.
    let saw_del = valid.contains(&0x7f);
    valid.retain(|&b| b != 0x7f);
    let mut out = Vec::with_capacity(valid.len());
    let mut context = Context { rep_never: saw_del, rows, ..Context::default() };
    let mut i = 0;
    while i < valid.len() {
        let byte = valid[i];
        if byte != 0x1b {
            match byte {
                // SO and SI: REP repeats what was displayed, not what would be drawn now.
                0x0e => (context.shifted, context.rep_unsafe) = (true, true),
                0x0f => (context.shifted, context.rep_unsafe) = (false, true),
                _ => {}
            }
            match byte {
                0x09 if context.after_text => {
                    i += 1;
                    continue;
                }
                0x08 | 0x0a..=0x0d => context.after_text = false,
                0x20.. => context.after_text = true,
                _ => {}
            }
            let (len, mark) = text_len(&valid[i..]);
            context.rep_unsafe = mark || (context.rep_unsafe && byte < 0x20);
            out.extend_from_slice(&valid[i..i + len]);
            i += len;
            continue;
        }
        let rest = &valid[i + 1..];
        // A control or a non-ASCII character right after ESC (or after the intermediates, or the
        // P of a DCS): the parsers disagree on whether the escape goes on.
        let after_inter = rest.iter().position(|b| !(0x20..=0x2f).contains(b)).unwrap_or(rest.len());
        let odd =
            |b: Option<&u8>| b.is_some_and(|b| (*b < 0x20 && !matches!(b, 0x18 | 0x1a | 0x1b)) || *b >= 0x80);
        if odd(rest.get(after_inter)) || (rest.first() == Some(&b'P') && odd(rest.get(1))) {
            i += 1;
        } else if let Some(skip) = string_to_drop(rest) {
            i += 1 + skip;
        } else if let Some(len) = string_len(rest) {
            // A string passes as it is; its text is not text. PM, SOS and APC strings go: the DEC
            // parser ignores everything in them, alacritty's executes the controls.
            if rest[0] == b']' {
                // The DEC parser ignores the controls in an OSC (BEL ends it), alacritty's
                // executes them.
                out.push(0x1b);
                out.extend(
                    rest[..len]
                        .iter()
                        .copied()
                        .filter(|b| *b >= 0x20 || matches!(b, 0x07 | 0x18 | 0x1a | 0x1b)),
                );
            } else if rest[0] == b'P' && rest[..len].iter().any(|b| *b >= 0x80) {
                // Text in a device control string: alacritty prints some of it.
            } else if rest[0] == b'P' {
                // The controls in a DCS are its data (Williams' passthrough state); alacritty's
                // executes them.
                out.push(0x1b);
                out.extend(
                    rest[..len].iter().copied().filter(|b| *b >= 0x20 || matches!(b, 0x18 | 0x1a | 0x1b)),
                );
            } else if !matches!(rest[0], b'^' | b'X' | b'_') {
                out.extend_from_slice(&valid[i..i + 1 + len]);
            }
            i += 1 + len;
        } else if rest.first() == Some(&b'[') {
            let (end, keep) = csi(&valid[i..], &mut context);
            out.extend_from_slice(&keep);
            i += end;
        } else {
            let inter_end = rest.iter().position(|b| !(0x20..=0x2f).contains(b)).unwrap_or(rest.len());
            match rest.get(inter_end) {
                Some(&fin) if (0x30..=0x7e).contains(&fin) && !b"[]P_^X".contains(&fin) => {
                    let end = i + 1 + inter_end + 1;
                    if esc_kept(&rest[..inter_end], fin, &mut context) {
                        out.extend_from_slice(&valid[i..end]);
                    }
                    i = end;
                }
                _ => {
                    out.push(0x1b);
                    i += 1;
                }
            }
        }
    }
    out
}

fn valid_utf8(stream: &[u8]) -> Vec<u8> {
    let mut valid = Vec::with_capacity(stream.len());
    let mut rest = stream;
    loop {
        match std::str::from_utf8(rest) {
            Ok(_) => {
                valid.extend_from_slice(rest);
                return valid;
            }
            Err(error) => {
                let good = error.valid_up_to();
                valid.extend_from_slice(&rest[..good]);
                rest = &rest[good + error.error_len().unwrap_or(rest.len() - good)..];
            }
        }
    }
}

/// The length of the character at the start of `text`, and whether it is a mark that joins the
/// character before it.
fn text_len(text: &[u8]) -> (usize, bool) {
    let len = match text[0] {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
    .min(text.len());
    let mark = std::str::from_utf8(&text[..len])
        .ok()
        .and_then(|s| s.chars().next())
        .is_some_and(|c| ('\u{300}'..='\u{36f}').contains(&c) || c == '\u{200d}');
    (len, mark)
}

/// DECRQSS and XTGETTCAP (alacritty does not answer them) and OSC 0/2 with no text (the new
/// engine takes it for a reset of the title, alacritty sets an empty one): how many bytes after
/// the ESC to drop.
fn string_to_drop(rest: &[u8]) -> Option<usize> {
    if rest.starts_with(b"P$q") || rest.starts_with(b"P+q") {
        return Some(match rest.iter().skip(1).position(|&b| b == 0x1b) {
            Some(at) => (at + 1) + 1 + usize::from(rest.get(at + 2) == Some(&b'\\')),
            None => rest.len(),
        });
    }
    if let Some(body) = rest.strip_prefix(b"]0;").or_else(|| rest.strip_prefix(b"]2;")) {
        // alacritty trims a title and takes blanks for none; the new engine keeps what was sent.
        let end = body.iter().position(|&b| b == 0x07 || b == 0x1b).unwrap_or(body.len());
        let title = &body[..end];
        if title.first() == Some(&b' ') || title.last() == Some(&b' ') || title.is_empty() {
            return Some(
                3 + end
                    + match body.get(end) {
                        Some(0x07) => 1,
                        Some(_) => 1 + usize::from(body.get(end + 1) == Some(&b'\\')),
                        None => 0,
                    },
            );
        }
    }
    // An OSC that an ESC (not the one of ST) interrupts: the DEC parser ends it there and acts
    // on it, alacritty's does not. Dropped, up to the ESC.
    if rest.first() == Some(&b']') {
        let end = rest.iter().position(|&b| b == 0x1b || b == 0x07)?;
        if rest[end] == 0x1b && rest.get(end + 1) != Some(&b'\\') {
            return Some(end);
        }
    }
    None
}

/// The length, after the ESC, of an OSC, DCS, APC, PM or SOS string to its end: its terminator
/// (BEL ends only an OSC), or the ESC that interrupts it, or the end of the stream.
fn string_len(rest: &[u8]) -> Option<usize> {
    let kind = *rest.first().filter(|b| b"]P_^X".contains(b))?;
    let mut at = 1;
    while at < rest.len() {
        match rest[at] {
            0x07 if kind == b']' => return Some(at + 1),
            0x1b if rest.get(at + 1) == Some(&b'\\') => return Some(at + 2),
            0x1b => return Some(at),
            _ => at += 1,
        }
    }
    Some(rest.len())
}

/// An ESC sequence with a final byte: kept or not.
fn esc_kept(inter: &[u8], fin: u8, context: &mut Context) -> bool {
    match (inter, fin) {
        // RIS: the stream starts over.
        ([], b'D' | b'E' | b'M') => {
            context.after_text = false;
            true
        }
        ([], b'c') => {
            *context = Context {
                rep_never: context.rep_never,
                rows: context.rows,
                rep_unsafe: true,
                ..Context::default()
            };
            true
        }
        // DECRC after a locking shift or origin mode: alacritty does not bring them back.
        ([], b'8') => {
            context.rep_unsafe = true;
            !context.shifted && !context.shift_saved && !context.origin
        }
        ([], b'7') => {
            context.shift_saved = context.shifted;
            true
        }
        // DECALN (alacritty does not home the cursor or reset the margins), DECBI, DECFI, the
        // single and locking shifts beyond SO and SI, DECID (the reply names the terminal).
        ([b'#'], b'8') | ([], b'6' | b'9' | b'n' | b'o' | b'N' | b'O' | b'Z') => false,
        // The UK set: alacritty keeps the number sign.
        ([b'(' | b')' | b'*' | b'+'], b'A') => false,
        // A character set designation: REP repeats what was displayed, not what is drawn now.
        // Only ASCII, the UK set and DEC special graphics are mapped by both.
        ([b'(' | b')' | b'*' | b'+'], b'B' | b'0') => {
            context.rep_unsafe = true;
            true
        }
        ([b'(' | b')' | b'*' | b'+'], _) => false,
        _ => true,
    }
}

/// A CSI at the start of `seq`: how many bytes it is and what to write for it.
fn csi(seq: &[u8], context: &mut Context) -> (usize, Vec<u8>) {
    let body = &seq[2..];
    let marker = body.first().copied().filter(|b| (b'<'..=b'?').contains(b));
    let start = usize::from(marker.is_some());
    let params_end =
        body[start..].iter().position(|b| !(b'0'..=b';').contains(b)).map_or(body.len(), |n| n + start);
    let inter_end = body[params_end..]
        .iter()
        .position(|b| !(0x20..=0x2f).contains(b))
        .map_or(body.len(), |n| n + params_end);
    // A private marker after the parameters makes the sequence invalid: the DEC parser reads it
    // to its final byte and does nothing, alacritty's does what the final byte says.
    if body.get(inter_end).is_some_and(|b| (0x3a..=0x3f).contains(b)) {
        let tail = body[inter_end..].iter().position(|b| (0x40..=0x7e).contains(b));
        if let Some(n) = tail {
            return (2 + inter_end + n + 1, Vec::new());
        }
    }
    let Some(&fin) = body.get(inter_end).filter(|b| (0x40..=0x7e).contains(*b)) else {
        // Cut short by whatever follows: what was read is a sequence that never happened, and
        // both engines throw it away; it must not turn into text here.
        // A control in the middle (the DEC parser executes it and goes on, alacritty's cancels
        // the sequence): the start of the sequence goes, the control stays and what follows is text.
        let by_control = body
            .get(inter_end)
            .is_some_and(|b| (*b < 0x20 && !matches!(b, 0x18 | 0x1a | 0x1b)) || *b >= 0x80);
        let raw = if by_control { Vec::new() } else { seq[..2 + inter_end].to_vec() };
        return (2 + inter_end, raw);
    };
    let params = std::str::from_utf8(&body[start..params_end]).unwrap_or("");
    let inter = &body[params_end..inter_end];
    let end = 2 + inter_end + 1;
    let raw = seq[..end].to_vec();
    let first = params.split([';', ':']).next().unwrap_or("");
    let plain = marker.is_none() && inter.is_empty();
    // Cursor movement bounded by the scrolling region.
    if plain && matches!(fin, b'A' | b'B' | b'E' | b'F' | b'e') && context.region {
        return (end, Vec::new());
    }
    if plain && matches!(fin, b'H' | b'f' | b'A' | b'B' | b'C' | b'D' | b'E' | b'F' | b'G' | b'd' | b'a' | b'e' | b'`')
    {
        context.after_text = false;
    }
    if plain && fin == b'I' && context.after_text {
        return (end, Vec::new());
    }
    if plain && fin == b'Z' && context.tabs_cleared {
        return (end, Vec::new());
    }
    if plain && fin == b'b' && (context.rep_unsafe || context.rep_never) {
        return (end, Vec::new());
    }
    if plain && fin == b'g' && matches!(first, "" | "0" | "3") {
        context.tabs_cleared = true;
    }
    if plain && fin == b'r' {
        // A region of less than two lines is not one: the new engine ignores the sequence
        // altogether, alacritty homes the cursor.
        let mut numbers = params.split(';').map(|p| p.parse::<usize>().unwrap_or(0));
        let top = numbers.next().unwrap_or(0).max(1);
        let explicit = numbers.next().filter(|&b| b != 0);
        let bottom = explicit.unwrap_or(context.rows).min(context.rows);
        if top >= bottom || (context.origin && explicit.is_none()) {
            return (end, Vec::new());
        }
        context.bottomless = explicit.is_none() && top > 1;
        context.region = !params.is_empty();
    }
    if plain && fin == b's' && params.is_empty() {
        context.shift_saved = context.shifted;
    }
    // SCOSC takes no parameters, and SD with two is xterm's mouse tracking.
    if plain
        && ((fin == b's' && !params.is_empty() && !params.contains(';'))
            || (fin == b'T' && params.contains(';')))
    {
        return (end, Vec::new());
    }
    // `?` with a final byte that only has a meaning without it: alacritty ignores the marker.
    if marker == Some(b'?')
        && inter.is_empty()
        && !matches!(fin, b'h' | b'l' | b'J' | b'K' | b'n' | b'W' | b'S' | b'u' | b'r' | b's')
    {
        return (end, Vec::new());
    }
    // Private markers other than `?`: alacritty runs the sequence without them (`CSI = K` is
    // EL); the specs make them other functions.
    if matches!(marker, Some(b'<' | b'=' | b'>')) && !(marker == Some(b'>') && fin == b'm') {
        return (end, Vec::new());
    }
    // Erases with a parameter that names no extent: nothing, in the new engine.
    if plain
        && ((fin == b'J' && first.parse::<u32>().is_ok_and(|n| n > 3))
            || (fin == b'K' && first.parse::<u32>().is_ok_and(|n| n > 2)))
    {
        return (end, Vec::new());
    }
    // CHA and HPA in origin mode: alacritty adds the region's top to the cursor's row again.
    if plain && matches!(fin, b'G' | b'`') && context.origin {
        return (end, Vec::new());
    }
    // CPR in origin mode: alacritty reports the absolute row.
    if plain && fin == b'n' && context.origin && params == "6" {
        return (end, Vec::new());
    }
    if plain && fin == b'u' {
        context.rep_unsafe = true;
    }
    if plain
        && fin == b'u'
        && (context.shifted || context.shift_saved || context.origin || !params.is_empty())
    {
        // SCORC takes no parameters; alacritty restores the cursor whatever they are.
        return (end, Vec::new());
    }
    if plain && fin == b'm' {
        return (
            end,
            sgr(params).map_or(raw, |kept| {
                if kept.is_empty() { Vec::new() } else { format!("\x1b[{kept}m").into_bytes() }
            }),
        );
    }
    if marker == Some(b'?') && inter.is_empty() && fin == b'h' && params.split(';').any(|p| p == "6") {
        if context.bottomless {
            return (end, Vec::new());
        }
        context.origin = true;
    }
    if marker == Some(b'?') && inter.is_empty() && matches!(fin, b'h' | b'l') {
        for p in params.split(';') {
            // Autowrap off, then on again: alacritty sets the wrap flag while it is off.
            if p == "7" {
                if fin == b'h' && context.no_wrap {
                    return (end, Vec::new());
                }
                context.no_wrap |= fin == b'l';
            }
            // The alternate screen restores the locking shift in the new engine (it is what DECSC
            // saves), not in alacritty.
            if p == "1049" {
                if fin == b'h' {
                    context.shift_saved = context.shifted;
                } else if context.shifted || context.shift_saved {
                    return (end, Vec::new());
                }
            }
            if p == "1049" {
                if fin == b'h' && context.alt {
                    return (end, Vec::new());
                }
                context.alt = fin == b'h';
            }
        }
    }
    if dropped(marker, params, inter, fin) {
        return (end, Vec::new());
    }
    // A mode change that resets the cursor in alacritty's way only.
    (end, raw)
}

/// Whether alacritty does not do this CSI as the specs say.
fn dropped(marker: Option<u8>, params: &str, inter: &[u8], fin: u8) -> bool {
    let first = params.split([';', ':']).next().unwrap_or("");
    match (marker, inter, fin) {
        // DECSTR, DECIC/DECDC, DECST8C, DA3, XTVERSION, DECSLRM: not implemented.
        (None, [b'!'], b'p') | (None, [b'\''], b'}' | b'~') | (Some(b'?'), [], b'W') => true,
        (Some(b'='), [], b'c') | (Some(b'>'), [], b'q') => true,
        (None, [], b's') => params.contains(';'),
        // DA1 and DA2 name the terminal: the engine's choice.
        (None | Some(b'>'), [], b'c') => true,
        // DECXCPR, printer status and the like, the cell size report.
        (Some(b'?'), [], b'n') => true,
        (None, [], b't') => first == "16",
        // DECRQM of modes alacritty does not know.
        (None, [b'$'], b'p') => !matches!(first, "4" | "20"),
        (Some(b'?'), [b'$'], b'p') => !matches!(first, "7" | "25" | "1049" | "9999" | "1" | "6"),
        // The modes alacritty ignores or handles differently: DECSCNM, the older alternate
        // screens, 1048, DECLRMM, and DECOM reset, which does not home the cursor there.
        (Some(b'?'), [], b'h') => params.split(';').any(|p| matches!(p, "5" | "47" | "69" | "1047" | "1048")),
        (Some(b'?'), [], b'l') => {
            params.split(';').any(|p| matches!(p, "5" | "6" | "47" | "69" | "1047" | "1048"))
        }
        // LNM: alacritty's LF does not return the carriage.
        (None, [], b'h' | b'l') => params.split(';').any(|p| p == "20"),
        // The selective erasers, ED 1 and ED 2 (alacritty pushes a blank screen into the
        // history), DCH.
        (Some(b'?'), [], b'J' | b'K') => true,
        (None, [], b'J') => matches!(first, "1" | "2"),
        (None, [], b'P') => true,
        _ => false,
    }
}

/// The SGR parameters without what alacritty does not do: blink (5, 6), double underline (21),
/// overline (53), colon forms other than the color and underline-style ones, and an extended
/// color whose kind is neither 5 nor 2, which no text defines (ECMA-48 reserves 38 and 48).
/// `None` when nothing needs to go; an empty string when nothing is left to say.
fn sgr(params: &str) -> Option<String> {
    let parts: Vec<&str> = params.split(';').collect();
    let mut kept: Vec<&str> = Vec::new();
    let mut changed = false;
    let mut i = 0;
    while i < parts.len() {
        let p = parts[i];
        if p.contains(':') {
            let mut fields = p.split(':');
            let head = fields.next().unwrap_or("");
            let style = fields.next().unwrap_or("");
            let fine = (matches!(head, "38" | "48" | "58") && matches!(style, "2" | "5"))
                || (head == "4"
                    && matches!(style, "0" | "1" | "2" | "3" | "4" | "5")
                    && fields.next().is_none());
            if fine {
                kept.push(p);
            } else {
                changed = true;
            }
            i += 1;
        } else if matches!(p, "38" | "48" | "58") {
            let take = match parts.get(i + 1).copied() {
                Some("5") => 3,
                Some("2") => 5,
                _ => return Some(String::new()),
            };
            let end = (i + take).min(parts.len());
            kept.extend_from_slice(&parts[i..end]);
            i = end;
        } else {
            if matches!(p, "5" | "6" | "21" | "53") {
                changed = true;
            } else {
                kept.push(p);
            }
            i += 1;
        }
    }
    changed.then(|| kept.join(";"))
}

#[cfg(test)]
mod tests {
    fn sanitize6(stream: &[u8]) -> Vec<u8> {
        super::sanitize(stream, 6)
    }

    #[test]
    fn the_sanitizer_drops_what_it_lists() {
        assert_eq!(sanitize6(b"a\x1b[48;4mb"), b"ab");
        assert_eq!(sanitize6(b"a\x1b[1;5;3mb"), b"a\x1b[1;3mb");
        assert_eq!(sanitize6(b"a\x1b[48;5;4;5mb"), b"a\x1b[48;5;4mb");
        assert_eq!(sanitize6(b"a\x1b[38;2;5;5;5mb"), b"a\x1b[38;2;5;5;5mb");
        assert_eq!(sanitize6(b"a\x1b[4:3;2:mb"), b"a\x1b[4:3mb");
        assert_eq!(sanitize6(b"a\x1b#8b\x1bMc"), b"ab\x1bMc");
        assert_eq!(sanitize6(b"a\x1bP$qm\x1b\\b"), b"ab");
        assert_eq!(sanitize6(b"a\x1b]0;\x07b\x1b]2;x\x07"), b"ab\x1b]2;x\x07");
        assert_eq!(sanitize6(b"a\x80b"), b"ab");
        assert_eq!(sanitize6(b"\x1b[2;4r\x1b[3A\x1b[r\x1b[3A"), b"\x1b[2;4r\x1b[r\x1b[3A");
        assert_eq!(sanitize6(b"a\x7f\x1b[3bb\x1b[3b"), b"ab");
        assert_eq!(sanitize6("a\u{301}\x1b[3bb\x1b[3b".as_bytes()), "a\u{301}b\x1b[3b".as_bytes());
    }
}

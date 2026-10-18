//! Everything besides the cells: tab stops, colors, titles, the keyboard stacks, the locator, the
//! modes, the saved cursors and the cursor itself, in the order that lets each sequence do what it
//! says (a mode that moves the cursor goes before the cursor is placed; the locator before the
//! mouse modes that would switch it off).

use crate::grid::{Content, Grid, Style};
use crate::types::{CursorShape, Flags, Line, NamedColor, Rgb};

use super::super::Cursor;
use super::super::charset::Charsets;
use super::super::conformance::Level;
use super::super::modes::{self, Modes};
use super::super::presentation::RESTORE_MAX;
use super::{Gaps, Writer};

/// The DEC private modes that are one flag and nothing else, with their numbers. The ones that do
/// more than set a flag (the screens, DECCOLM, DECOM, the locator) are written by other means.
const PRIVATE: &[(Modes, u16)] = &[
    (Modes::SHOW_CURSOR, 25),
    (Modes::AUTOWRAP, 7),
    (Modes::ALTERNATE_SCROLL, 1007),
    (Modes::BRACKETED_PASTE, 2004),
    (Modes::FOCUS_REPORT, 1004),
    (Modes::APP_CURSOR, 1),
    (Modes::MOUSE_CLICK, 1000),
    (Modes::MOUSE_DRAG, 1002),
    (Modes::MOUSE_MOTION, 1003),
    (Modes::MOUSE_SGR, 1006),
    (Modes::MOUSE_UTF8, 1005),
    (Modes::SMOOTH_SCROLL, 4),
    (Modes::REVERSE_VIDEO, 5),
    (Modes::AUTO_REPEAT, 8),
    (Modes::MOUSE_X10, 9),
    (Modes::ALLOW_COLUMNS, 40),
    (Modes::MORE_FIX, 41),
    (Modes::REVERSE_WRAP, 45),
    (Modes::APP_KEYPAD, 66),
    (Modes::BACKARROW_BS, 67),
    (Modes::LEFT_RIGHT_MARGINS, 69),
    (Modes::SIXEL_DISPLAY, 80),
    (Modes::NO_CLEAR_ON_COLUMNS, 95),
    (Modes::MOUSE_URXVT, 1015),
    (Modes::MOUSE_PIXELS, 1016),
    (Modes::SYNC_OUTPUT, 2026),
    (Modes::GRAPHEME_CLUSTERS, 2027),
    (Modes::SIXEL_PRIVATE_PALETTE, 1070),
    (Modes::SIXEL_CURSOR_RIGHT, 8452),
    (Modes::THEME_REPORT, 2031),
    (Modes::SCROLL_ON_OUTPUT, 1010),
    (Modes::SCROLL_ON_KEY, 1011),
    (Modes::META_SENDS_ESC, 1036),
    (Modes::ALT_SENDS_ESC, 1039),
    (Modes::BELL_URGENCY, 1042),
    (Modes::REVERSE_WRAP_EXTENDED, 1045),
];

/// The ANSI modes, with their numbers.
const ANSI: &[(Modes, u16)] =
    &[(Modes::KEYBOARD_LOCK, 2), (Modes::INSERT, 4), (Modes::SEND_RECEIVE, 12), (Modes::NEW_LINE, 20)];

/// Modes that move the cursor or the screen when they change, which the XTSAVE table cannot be
/// built with.
const NOT_SAVEABLE: &[u16] = &[3, 47, 1047, 1048, 1049];

impl<'a> Writer<'a> {
    pub(super) fn tables(&mut self) {
        self.tab_stops();
        self.palette();
        self.titles();
        self.keyboard();
        self.saved_modes();
        self.misc();
        self.locator();
    }

    // ---- tab stops ----

    fn tab_stops(&mut self) {
        let t = self.term;
        // A stop every eighth column is what a fresh receiver has.
        if t.tabs.columns().eq((0..t.cols).filter(|x| x % 8 == 0 && *x > 0)) {
            return;
        }
        if t.level >= Level::Vt300 {
            let start = self.out.len();
            self.put(b"\x1bP2$t");
            let payload = self.out.len();
            for (i, column) in t.tabs.columns().enumerate() {
                if i > 0 {
                    self.out.push(b'/');
                }
                self.num(column + 1);
            }
            // The receiver drops a longer DECRSPS whole: thousands of stops on a very wide screen.
            if self.out.len() - payload <= RESTORE_MAX {
                self.put(b"\x1b\\");
                return;
            }
            self.out.truncate(start);
        }
        // TBC 3, then HTS where each stop is.
        self.put(b"\x1b[3g");
        for column in t.tabs.columns() {
            self.csi2(1, column + 1, b'H');
            self.put(b"\x1bH");
        }
    }

    // ---- colors ----

    fn palette(&mut self) {
        let t = self.term;
        let spec = |out: &mut Vec<u8>, rgb: Rgb| {
            const HEX: &[u8; 16] = b"0123456789abcdef";
            out.extend_from_slice(b"rgb:");
            for (k, byte) in [rgb.r, rgb.g, rgb.b].into_iter().enumerate() {
                if k > 0 {
                    out.push(b'/');
                }
                out.push(HEX[usize::from(byte >> 4)]);
                out.push(HEX[usize::from(byte & 15)]);
            }
        };
        // OSC 4 takes pairs; sixteen at a time keeps every string short.
        let mut open = 0;
        for i in 0..256 {
            let Some(rgb) = t.colors.get(i) else { continue };
            if open == 0 {
                self.put(b"\x1b]4");
            }
            self.out.push(b';');
            self.num(i);
            self.out.push(b';');
            spec(self.out, rgb);
            open += 1;
            if open == 16 {
                self.put(b"\x1b\\");
                open = 0;
            }
        }
        if open > 0 {
            self.put(b"\x1b\\");
        }
        for (i, color) in t.special_colors.iter().enumerate() {
            if let Some(rgb) = color {
                self.put(b"\x1b]5;");
                self.num(i);
                self.out.push(b';');
                spec(self.out, *rgb);
                self.put(b"\x1b\\");
            }
        }
        for (command, named) in
            [(10, NamedColor::Foreground), (11, NamedColor::Background), (12, NamedColor::Cursor)]
        {
            if let Some(rgb) = t.colors[named] {
                self.put(b"\x1b]");
                self.num(command);
                self.out.push(b';');
                spec(self.out, rgb);
                self.put(b"\x1b\\");
            }
        }
        for (i, color) in t.extra_colors.iter().enumerate() {
            if let Some(rgb) = color {
                self.put(b"\x1b]");
                self.num(13 + i);
                self.out.push(b';');
                spec(self.out, *rgb);
                self.put(b"\x1b\\");
            }
        }
    }

    // ---- titles ----

    /// Whether `text` can be the body of an OSC string.
    fn osc_safe(&mut self, text: &str) -> bool {
        let ok = !text.chars().any(|c| c.is_control() && !('\u{80}'..='\u{9f}').contains(&c));
        if !ok {
            self.gaps |= Gaps::TEXT;
        }
        ok
    }

    /// `OSC code ; text ST`; an empty text unsets.
    fn osc_text(&mut self, code: &str, text: Option<&str>) {
        let text = text.unwrap_or("");
        if !self.osc_safe(text) {
            return self.osc_text(code, None);
        }
        self.put(b"\x1b]");
        self.put(code.as_bytes());
        self.out.push(b';');
        self.put(text.as_bytes());
        self.put(b"\x1b\\");
    }

    fn titles(&mut self) {
        let t = self.term;
        for slot in &t.title_stack {
            if let Some(window) = &slot.window {
                self.osc_text("2", window.as_deref());
            }
            if let Some(icon) = &slot.icon {
                self.osc_text("1", icon.as_deref());
            }
            let which = match (slot.window.is_some(), slot.icon.is_some()) {
                (true, true) => 0,
                (false, true) => 1,
                _ => 2,
            };
            self.put(b"\x1b[22;");
            self.num(which);
            self.put(b"t");
        }
        self.osc_text("2", t.title.as_deref());
        self.osc_text("1", t.icon_name.as_deref());
        if !t.cwd.is_empty() {
            self.osc_text("7", Some(&t.cwd));
        }
        // The modes come after the strings: from here on titles are hexadecimal.
        for (flag, param) in t.title_modes.parts() {
            if flag {
                self.put(b"\x1b[>");
                self.num(param);
                self.put(b"t");
            }
        }
    }

    // ---- the keyboard ----

    fn keyboard(&mut self) {
        let t = self.term;
        for screen in 0..2 {
            let stack = t.keys[screen].entries();
            if stack.len() == 1 && stack[0] == 0 {
                continue;
            }
            self.show(screen == 1);
            // The base entry is set, the rest pushed.
            self.put(b"\x1b[=");
            self.num(usize::from(stack[0]));
            self.put(b";1u");
            for &flags in &stack[1..] {
                self.put(b"\x1b[>");
                self.num(usize::from(flags));
                self.put(b"u");
            }
        }
        self.show(false);
    }

    // ---- the locator ----

    fn locator(&mut self) {
        let l = self.term.locator.settings();
        if l.reporting == 0 && !l.pixels && !l.on_down && !l.on_up {
            return;
        }
        // DECELR forgets the filter and takes the pointer from the mouse modes.
        self.put(b"\x1b[");
        self.num(usize::from(l.reporting));
        self.put(b";");
        self.num(usize::from(l.pixels));
        self.put(b"'z");
        if l.on_down {
            self.put(b"\x1b[1'{");
        }
        if l.on_up {
            self.put(b"\x1b[3'{");
        }
        if let Some([top, left, bottom, right]) = l.filter {
            self.put(b"\x1b[");
            for (i, edge) in [top, left, bottom, right].into_iter().enumerate() {
                if i > 0 {
                    self.out.push(b';');
                }
                self.num(edge as usize);
            }
            self.put(b"'w");
        }
    }

    // ---- what is left ----

    fn misc(&mut self) {
        let t = self.term;
        // DECAUPSS.
        if t.preferred != Default::default() {
            self.put(b"\x1bP1!uA\x1b\\");
        }
        // Numbers programs set for hardware we do not have.
        let kept = t.kept;
        for (value, tail) in [
            (kept.local_functions, &b"+q"[..]),
            (kept.modifier_keys, b"+r"),
            (kept.function_key_control, b"*}"),
            (kept.status_display, b"$}"),
            (kept.status_line, b"$~"),
            (kept.attribute_extent, b"*x"),
        ] {
            if value != 0 {
                self.put(b"\x1b[");
                self.num(usize::from(value));
                self.put(tail);
            }
        }
        // SPA then EPA: the receiver learns that guarded areas are in use.
        if t.guard_used {
            self.put(b"\x1bV\x1bW");
        }
        // The cursor's look: DECSCUSR sets the shape and its blink, mode 12 the blink after it.
        if let Some(shape) = t.cursor_shape {
            let blink = t.cursor_blink.unwrap_or(false);
            let base = match shape {
                CursorShape::Block => 1,
                CursorShape::Underline => 3,
                CursorShape::Beam => 5,
                #[allow(unreachable_patterns)]
                _ => 1,
            };
            self.put(b"\x1b[");
            self.num(base + usize::from(!blink));
            self.put(b" q");
        }
        if let Some(blink) = t.cursor_blink {
            self.private_mode(12, blink);
        } else if t.cursor_shape.is_none() && self.touched.contains(&12) {
            // The XTSAVE replay set the blink: DECSCUSR 0 hands it back to the host.
            self.put(b"\x1b[0 q");
        }
    }

    // ---- XTSAVE ----

    /// The modes XTSAVE put aside, in the order it did: each is made what it was and saved.
    /// Whatever they leave changed is written again by [`Writer::modes`].
    fn saved_modes(&mut self) {
        let t = self.term;
        for &(n, switch) in &t.saved_modes {
            let on = switch.is_set();
            if NOT_SAVEABLE.contains(&n) && n != 3 {
                // The alternate screen modes report which screen shows: be on that one.
                let want_alt = on;
                if matches!(n, 47 | 1047 | 1049) {
                    self.show(want_alt);
                    self.put(b"\x1b[?");
                    self.num(usize::from(n));
                    self.put(b"s");
                    continue;
                }
            }
            if n == 3 || n == 1048 {
                // DECCOLM resizes; 1048 is only ever reported reset.
                if on {
                    self.gaps |= Gaps::SAVED_MODES;
                } else {
                    self.put(b"\x1b[?");
                    self.num(usize::from(n));
                    self.put(b"s");
                }
                continue;
            }
            let known = modes::private_flag(n).is_some() || modes::recorded_bit(n).is_some() || n == 12;
            if known {
                self.private_mode(n, on);
                self.touched.push(n);
            } else if on {
                // A mode the engine does not know is never reported set.
                self.gaps |= Gaps::SAVED_MODES;
            }
            self.put(b"\x1b[?");
            self.num(usize::from(n));
            self.put(b"s");
        }
        self.show(false);
    }

    // ---- the counters and the cell `last_char` lives in ----

    /// `last_char`, and the block counter if the rows left it short of the sender's: both are only
    /// set by printing and by prompt marks, so they are done on a blank cell that is erased after.
    pub(super) fn touch_and_counters(&mut self) {
        let t = self.term;
        let end = t.block;
        let burns = if t.pen.block == 0 {
            // The pen belongs to no block: nothing may have been marked, and the counter is seeded.
            if self.seed_block.is_some() {
                self.gaps |= Gaps::BLOCKS;
            } else {
                self.seed_block = Some(end);
                self.counter = end;
            }
            0
        } else {
            if t.pen.block != end {
                self.gaps |= Gaps::BLOCKS;
            }
            let seed = *self.seed_block.get_or_insert_with(|| end.saturating_sub(1));
            self.counter = self.counter.max(seed);
            if self.counter > end {
                self.gaps |= Gaps::BLOCKS;
            }
            end.saturating_sub(self.counter) as usize
        };
        let reprinted = t.cluster.open && !self.has(Modes::INSERT);
        let last = t.last_char.filter(|_| !reprinted);
        if burns == 0 && last.is_none() {
            return;
        }
        self.reset_pen();
        let primary = if t.on_alt_screen() { &t.other } else { &t.grid };
        let Some(c) = last else {
            self.burn(burns);
            return;
        };
        // A cell that already shows the character is printed again, as it is: it sets
        // `last_char` and changes nothing else.
        if let Some((y, x)) = find_char(primary, c, (t.cursor.x, t.cursor.y))
            && let Some(view) = primary.row_view(Line(y as i32))
        {
            let link = view.links().find(|&(lo, hi, _)| lo <= x && x < hi).map(|(_, _, link)| link);
            self.csi2(y + 1, x + 1, b'H');
            self.put_cell(&view, x, link);
            self.burn(burns);
            return;
        }
        // Otherwise it is printed on a blank cell, which is erased again: with the background it
        // had.
        let width = crate::unicode::GraphemeState::new().step(c).width.max(1) as usize;
        if let Some((y, x, bg)) = blank_run(primary, width) {
            self.csi2(y + 1, x + 1, b'H');
            let mut buf = [0u8; 4];
            self.put(c.encode_utf8(&mut buf).as_bytes());
            self.burn(burns);
            self.set_style(&bg.map_or(Style::DEFAULT, Style::erased));
            self.csi2(y + 1, x + 1, b'H');
            self.csi_count(width, b'X');
        } else {
            self.gaps |= Gaps::TOUCH;
            self.burn(burns);
        }
    }

    fn burn(&mut self, n: usize) {
        for _ in 0..n {
            self.osc("133;A");
            self.counter += 1;
        }
        if n > 0 {
            self.pen.block = self.counter;
            self.pen.content = Content::Prompt;
        }
    }

    // ---- cursors ----

    /// DECSC on each screen with the state it holds.
    pub(super) fn saved_cursors(&mut self) {
        let t = self.term;
        for screen in [1usize, 0] {
            let saved = &t.saved[screen];
            self.show(screen == 1);
            self.cursor_information(saved.cursor, saved.origin, &saved.charsets);
            self.set_style(&saved.style);
            self.set_protected(saved.protected);
            self.set_link(saved.link.as_ref());
            self.put(b"\x1b7");
        }
    }

    /// DECRSPS: the cursor's place, the delayed wrap, DECOM and the character sets in one string.
    /// Below VT300 the terminal has no such sequence and the place is the only part written.
    fn cursor_information(&mut self, cursor: Cursor, origin: bool, charsets: &Charsets) {
        let t = self.term;
        if t.level < Level::Vt300 {
            let default = Charsets::default();
            if cursor.pending || *charsets != default {
                self.gaps |= Gaps::LOW_LEVEL;
            }
            self.private_mode(6, false);
            self.csi2(cursor.y + 1, cursor.x + 1, b'H');
            if origin {
                self.private_mode(6, true);
            }
            return;
        }
        let (gl, gr) = charsets.invoked();
        let sizes = 0x40 | (0..4).map(|i| u8::from(charsets.slot(i).is_96()) << i).fold(0, |a, b| a | b);
        let single = charsets.single();
        let flags = 0x40
            | u8::from(origin)
            | u8::from(single == Some(2)) << 1
            | u8::from(single == Some(3)) << 2
            | u8::from(cursor.pending) << 3;
        self.put(b"\x1bP1$t");
        self.num(cursor.y + 1);
        self.out.push(b';');
        self.num(cursor.x + 1);
        // Page 1; rendition and protection are the pen's, written by SGR and DECSCA instead.
        self.put(b";1;@;@;");
        self.out.push(flags);
        self.out.push(b';');
        self.num(usize::from(gl));
        self.out.push(b';');
        self.num(usize::from(gr));
        self.out.push(b';');
        self.out.push(sizes);
        self.out.push(b';');
        for i in 0..4 {
            self.put(charsets.slot(i).designator().as_bytes());
        }
        self.put(b"\x1b\\");
        // The string sets the four attributes DECCIR reports and the protection to nothing.
        self.pen.style.attrs.remove(Flags::BOLD | Flags::UNDERLINE | Flags::BLINK | Flags::INVERSE);
        self.pen.protected = false;
    }

    // ---- modes ----

    /// Every mode that is not what a reset leaves, and every one the XTSAVE phase touched, then
    /// the margins they enable.
    pub(super) fn modes(&mut self) {
        let t = self.term;
        self.clusters_on();
        for &(flag, n) in PRIVATE {
            let want = t.modes.contains(flag);
            let init = Modes::INITIAL.contains(flag);
            let always = flag == Modes::ALLOW_COLUMNS || self.touched.contains(&n);
            if want != init || always {
                self.private_mode(n, want);
            }
        }
        for &(flag, n) in ANSI {
            let want = t.modes.contains(flag);
            if want != Modes::INITIAL.contains(flag) {
                self.put(b"\x1b[");
                self.num(usize::from(n));
                self.out.push(if want { b'h' } else { b'l' });
            }
        }
        for n in modes::recorded_modes(t.recorded) {
            self.private_mode(n, true);
        }
        if t.modes.contains(Modes::COLUMNS_132) {
            self.gaps |= Gaps::COLUMNS_132;
        }
        let m = t.margins;
        if (m.top, m.bottom) != (0, t.rows - 1) {
            self.csi2(m.top + 1, m.bottom + 1, b'r');
        }
        if t.modes.contains(Modes::LEFT_RIGHT_MARGINS) && (m.left, m.right) != (0, t.cols - 1) {
            self.csi2(m.left + 1, m.right + 1, b's');
        }
    }

    // ---- the end ----

    /// The screen that shows, the pen, the cursor and what depends on the last thing printed.
    pub(super) fn final_state(&mut self) {
        let t = self.term;
        let alt = t.on_alt_screen();
        // The mark of the cell the pen would print is a main-screen thing: set it while the main
        // screen shows, the switch takes it along.
        let content = if alt { t.stashed_content } else { t.pen.content };
        if content != Content::None && t.pen.block != 0 {
            self.want_content(t.pen.block, content);
        } else {
            self.want_content(0, Content::None);
        }
        if alt {
            self.show(true);
            if t.pen.content != Content::None {
                self.gaps |= Gaps::ALT_CONTENT;
            }
        }
        // The cursor, or the place the open cluster starts when the last character printed can
        // still be extended: it is printed again from there and leaves the cursor, the cluster
        // and `last_char` where they were.
        let reprint = t.cluster.open
            && if self.has(Modes::INSERT) || !self.cluster_matches() {
                self.gaps |= Gaps::CLUSTER;
                false
            } else {
                true
            };
        let origin = t.modes.contains(Modes::ORIGIN);
        let at = if reprint { Cursor { x: t.cluster.head, y: t.cursor.y, pending: false } } else { t.cursor };
        self.cursor_information(at, origin, &t.charsets);
        // The pen after the string: it clears the attributes it reports.
        self.set_style(&t.pen.style);
        self.set_link(t.pen.link.as_ref());
        self.set_protected(t.pen.protected);
        self.set_guarded(t.pen.guarded);
        if t.eight_bit {
            self.put(b"\x1b G");
        }
        if reprint {
            self.cluster_text(t.cluster.head, t.cursor.y);
        }
        // After ED 3 a cursor move or an erase is all that came: the next ED 2 saves nothing.
        if t.history_just_cleared {
            if t.grid.history_len() + t.other.history_len() == 0 {
                self.put(b"\x1b[3J");
            } else {
                self.gaps |= Gaps::HISTORY_CLEARED;
            }
        }
    }

    /// Whether the cell at the open cluster's head holds the cluster: its last character is the
    /// cluster's last (printing the cell again then leaves `last` and the width as they are).
    fn cluster_matches(&self) -> bool {
        let t = self.term;
        let Some(view) = t.grid.row_view(Line(t.cursor.y as i32)) else { return false };
        let Some(&cell) = view.cells().get(t.cluster.head) else { return false };
        let last = view.marks(t.cluster.head).last().copied().or(cell.character());
        last == Some(t.cluster.last) && !cell.is_spacer_tail()
    }

    /// The cluster at column `x` of row `y`, as the characters that make it.
    fn cluster_text(&mut self, x: usize, y: usize) {
        let t = self.term;
        let Some(view) = t.grid.row_view(crate::types::Line(y as i32)) else { return };
        let cell = view.cells()[x];
        let c = cell.character().unwrap_or(' ');
        let mut buf = [0u8; 4];
        self.put(c.encode_utf8(&mut buf).as_bytes());
        for &mark in view.marks(x) {
            self.put(mark.encode_utf8(&mut buf).as_bytes());
        }
    }
}

/// A cell of `grid`'s live screen that shows `c` as its character and no block mark, the nearest
/// to `cursor` (a character printed last is usually just before it).
fn find_char(grid: &Grid, c: char, cursor: (usize, usize)) -> Option<(usize, usize)> {
    let rows = grid.rows();
    let order = (0..rows)
        .flat_map(|d| [cursor.1.checked_sub(d), (d > 0).then(|| cursor.1 + d).filter(|y| *y < rows)]);
    for y in order.flatten() {
        let view = grid.row_view(Line(y as i32))?;
        let cells = view.cells();
        let found = (0..cells.len()).rev().find(|&x| {
            let cell = cells[x];
            cell.character() == Some(c) && cell.content() == Content::None && !cell.is_spacer_tail()
        });
        if let Some(x) = found {
            return Some((y, x));
        }
    }
    None
}

/// A row and column of `grid`'s live screen where `width` cells in a row hold nothing, have the
/// same background, and do not reach the last column; the background.
fn blank_run(grid: &Grid, width: usize) -> Option<(usize, usize, Option<crate::types::Color>)> {
    let (cols, rows) = (grid.cols(), grid.rows());
    for y in (0..rows).rev() {
        let view = grid.row_view(Line(y as i32))?;
        let cells = view.cells();
        for x in 0..cols.saturating_sub(width) {
            let run = &cells[x..x + width];
            if run.iter().any(|c| c.codepoint() != 0 || c.flags() != 0) {
                continue;
            }
            let style = view.style(run[0].style_id());
            let same = run.iter().all(|c| view.style(c.style_id()) == style);
            if same && (style.is_default() || *style == Style::erased(style.bg)) {
                return Some((y, x, (!style.is_default()).then_some(style.bg)));
            }
        }
    }
    None
}

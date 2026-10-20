//! Everything besides the cells and the cursor: tab stops, colors, titles, the keyboard stacks,
//! the locator, XTSAVE's table and what else a program set, in the order that lets each sequence
//! do what it says (the locator before the mouse modes that would switch it off). `cursors` writes
//! the counters, the saved cursors, the modes and the cursor itself.

use crate::types::{Colors, CursorShape, NamedColor, Rgb};

use super::{Gaps, Writer};
use crate::terminal::modes;
use crate::terminal::modes::conformance::Level;
use crate::terminal::reports::presentation::RESTORE_MAX;

/// Modes that move the cursor or the screen when they change, which the XTSAVE table cannot be
/// built with.
const NOT_SAVEABLE: &[u16] = &[3, 47, 1047, 1048, 1049];

impl<'a> Writer<'a> {
    pub(super) fn tables(&mut self) {
        self.tab_stops();
        self.palette();
        self.sgr_stack();
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

    /// The colors programs set, and the palettes they pushed: each pushed one is set and pushed
    /// again, the receiver's colors reset between them, and the colors in force are set last.
    fn palette(&mut self) {
        let t = self.term;
        let mut dirty = false;
        for saved in t.color_stack.entries() {
            if dirty {
                self.reset_palette();
            }
            self.write_palette(&saved.set, &saved.special, &saved.extra);
            self.put(b"\x1b[#P");
            dirty = true;
        }
        if dirty {
            self.reset_palette();
        }
        self.write_palette(&t.palette.set, &t.palette.special, &t.palette.extra);
    }

    /// Every color a program can set goes back to the host's: OSC 104 (the palette and the special
    /// colors) and OSC 110 to 119 (the dynamic colors).
    fn reset_palette(&mut self) {
        self.put(b"\x1b]104\x1b\\");
        for command in 110..=119 {
            self.put(b"\x1b]");
            self.num(command);
            self.put(b"\x1b\\");
        }
    }

    fn write_palette(&mut self, set: &Colors, special: &[Option<Rgb>], extra: &[Option<Rgb>]) {
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
            let Some(rgb) = set.get(i) else { continue };
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
        for (i, color) in special.iter().enumerate() {
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
            if let Some(rgb) = set[named] {
                self.put(b"\x1b]");
                self.num(command);
                self.out.push(b';');
                spec(self.out, rgb);
                self.put(b"\x1b\\");
            }
        }
        for (i, color) in extra.iter().enumerate() {
            if let Some(rgb) = color {
                self.put(b"\x1b]");
                self.num(13 + i);
                self.out.push(b';');
                spec(self.out, *rgb);
                self.put(b"\x1b\\");
            }
        }
    }

    // ---- the video attributes' stack ----

    /// XTPUSHSGR's entries: the pen is made what each saved and pushed, for the attributes it
    /// named. The pen the terminal has is set at the end, with the rest of its state.
    fn sgr_stack(&mut self) {
        let t = self.term;
        for entry in t.sgr_stack.entries() {
            self.set_style(&entry.style);
            self.put(b"\x1b[");
            for (i, number) in entry.mask.numbers().enumerate() {
                if i > 0 {
                    self.out.push(b';');
                }
                self.num(usize::from(number));
            }
            self.put(b"#{");
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
        for slot in &t.titles.stack {
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
        self.osc_text("2", t.titles.window.as_deref());
        self.osc_text("1", t.titles.icon.as_deref());
        if !t.folder.now.is_empty() {
            self.osc_text("7", Some(&t.folder.now));
        }
        // The modes come after the strings: from here on titles are hexadecimal.
        for (flag, param) in t.titles.modes.parts() {
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
        if let Some(shape) = t.cursor_style.shape {
            let blink = t.cursor_style.blink.unwrap_or(false);
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
        if let Some(blink) = t.cursor_style.blink {
            self.private_mode(12, blink);
        } else if t.cursor_style.shape.is_none() && self.touched.contains(&12) {
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
            let known =
                modes::private_flag(n).is_some() || modes::recorded_bit(n).is_some() || n == 12 || n == 2;
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
}

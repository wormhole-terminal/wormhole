//! OSC 22: a program asks for a mouse pointer shape.
//!
//! Spec: xterm's ctlseqs ("OSC 22 ; Pt": the name of a cursor from the X11 cursor font) and
//! kitty's pointer shape protocol (<https://sw.kovidgoyal.net/kitty/pointer-shapes/>), which
//! names the shapes after the CSS `cursor` keywords and allows a comma-separated list. The engine
//! reads the setting form (`OSC 22 ; name`, `OSC 22 ; =name`; empty gives the pointer back to the
//! host) and hands the host a CSS keyword, so a host maps one vocabulary. The names are checked
//! against the keyword list, the common X11 names xterm programs use are translated, and
//! anything else is dropped: a name is a program's text and nothing the host should have to
//! trust. The stack (`>` and `<`) and the queries (`?`) are not read: a host keeps a pointer, not
//! a history of them, and a program that asks gets no answer.

use crate::terminal::Terminal;
use crate::terminal::event::Event;

/// The longest name read, in bytes (the longest keyword is 14).
const NAME_MAX: usize = 32;

/// The CSS `cursor` keywords that name a shape (not `auto`, `none` or `url()`), as kitty's
/// protocol lists them.
const SHAPES: [&str; 30] = [
    "alias",
    "cell",
    "copy",
    "crosshair",
    "default",
    "e-resize",
    "ew-resize",
    "grab",
    "grabbing",
    "help",
    "move",
    "n-resize",
    "ne-resize",
    "nesw-resize",
    "no-drop",
    "not-allowed",
    "ns-resize",
    "nw-resize",
    "nwse-resize",
    "pointer",
    "progress",
    "s-resize",
    "se-resize",
    "sw-resize",
    "text",
    "vertical-text",
    "w-resize",
    "wait",
    "zoom-in",
    "zoom-out",
];

/// X11 cursor font names xterm programs use, and the shape each is.
const X11: [(&str, &str); 19] = [
    ("left_ptr", "default"),
    ("arrow", "default"),
    ("xterm", "text"),
    ("ibeam", "text"),
    ("hand1", "pointer"),
    ("hand2", "pointer"),
    ("watch", "wait"),
    ("fleur", "move"),
    ("cross", "crosshair"),
    ("question_arrow", "help"),
    ("sb_h_double_arrow", "ew-resize"),
    ("sb_v_double_arrow", "ns-resize"),
    ("top_side", "n-resize"),
    ("bottom_side", "s-resize"),
    ("left_side", "w-resize"),
    ("right_side", "e-resize"),
    ("top_left_corner", "nw-resize"),
    ("top_right_corner", "ne-resize"),
    ("bottom_left_corner", "sw-resize"),
];

/// The CSS keyword `name` stands for, if it is one this reads.
fn shape(name: &[u8]) -> Option<&'static str> {
    if name.is_empty() || name.len() > NAME_MAX {
        return None;
    }
    let name = std::str::from_utf8(name).ok()?;
    SHAPES
        .iter()
        .find(|shape| **shape == name)
        .copied()
        .or_else(|| X11.iter().find(|(x11, _)| *x11 == name).map(|&(_, css)| css))
}

impl Terminal {
    /// OSC 22 with `rest` after the number: the first name of the list that is a shape the host
    /// can draw, or the host's own pointer again for an empty one.
    pub(in crate::terminal) fn osc_pointer_shape(&mut self, rest: &[u8]) {
        let names = rest.strip_prefix(b"=").unwrap_or(rest);
        if matches!(names.first(), Some(b'>' | b'<' | b'?')) {
            return;
        }
        if names.is_empty() {
            self.push_event(Event::PointerShape(String::new()));
        } else if let Some(shape) = names.split(|&b| b == b',').find_map(shape) {
            self.push_event(Event::PointerShape(shape.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SHAPES, X11, shape};

    #[test]
    fn the_lists_are_in_the_vocabulary_they_translate_to() {
        for (x11, css) in X11 {
            assert!(SHAPES.contains(&css), "{x11} -> {css}");
            assert!(!SHAPES.contains(&x11), "{x11} is a keyword already");
        }
    }

    #[test]
    fn names_are_read_exactly() {
        assert_eq!(shape(b"pointer"), Some("pointer"));
        assert_eq!(shape(b"xterm"), Some("text"));
        for bad in [&b""[..], b"Pointer", b"auto", b"url(x)", b"point\xffer", &[b'a'; 40]] {
            assert_eq!(shape(bad), None, "{bad:?}");
        }
    }
}

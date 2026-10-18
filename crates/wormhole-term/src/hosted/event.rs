//! The events a hosted terminal's host tells its clients, owned (the wire's `Event` borrows), and the
//! mapping of window operations to and from the wire's three numbers.

use wormhole_vt::term::{Extent, Fullscreen, Maximize, WindowOp};
use wormhole_wire as wire;

/// One event of the program (or of the host), without the stream offset it happened at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostEvent {
    Title(String),
    ResetTitle,
    Bell,
    /// The folder the shell reported (OSC 7), already checked to be a folder of this machine.
    Cwd(String),
    Mark(wire::Mark),
    /// A program's notification, cleaned. `source` is 0: the engine does not say which OSC it was.
    Notify {
        source: u8,
        title: String,
        body: String,
    },
    Progress {
        state: wire::ProgressState,
        percent: u8,
    },
    /// The body of an accepted OSC 7501 report as the program wrote it (UTF-8, checked).
    ProgramStatus(String),
    /// OSC 52 write. `data` is the text (UTF-8), not base64.
    ClipboardStore {
        selection: u8,
        data: Vec<u8>,
    },
    ClipboardLoad {
        ticket: u32,
        selection: u8,
    },
    WindowOp {
        op: u8,
        a: u32,
        b: u32,
    },
    Exited(wire::ExitInfo),
    FullReset,
    /// The counters a repaint cannot say; sent right before one (see [`wire::EventKind::RepaintSeeds`]).
    RepaintSeeds {
        block: u32,
        link_seq: u64,
        image_handle: u64,
    },
}

impl HostEvent {
    /// The wire event for this one at stream `offset`.
    pub fn to_wire(&self, offset: u64) -> wire::Event<'_> {
        use wire::EventKind as K;
        let kind = match self {
            Self::Title(t) => K::Title(t),
            Self::ResetTitle => K::ResetTitle,
            Self::Bell => K::Bell,
            Self::Cwd(p) => K::Cwd(p),
            Self::Mark(m) => K::Mark(*m),
            Self::Notify { source, title, body } => K::Notify { source: *source, title, body },
            Self::Progress { state, percent } => K::Progress { state: *state, percent: *percent },
            Self::ProgramStatus(s) => K::ProgramStatus(s),
            Self::ClipboardStore { selection, data } => K::ClipboardStore { selection: *selection, data },
            Self::ClipboardLoad { ticket, selection } => {
                K::ClipboardLoad { ticket: *ticket, selection: *selection }
            }
            Self::WindowOp { op, a, b } => K::WindowOp { op: *op, a: *a, b: *b },
            Self::Exited(x) => K::Exited(*x),
            Self::FullReset => K::FullReset,
            Self::RepaintSeeds { block, link_seq, image_handle } => {
                K::RepaintSeeds { block: *block, link_seq: *link_seq, image_handle: *image_handle }
            }
        };
        wire::Event { offset, kind }
    }

    /// Whether only the controller hears it (the clipboard is the user's, once).
    pub fn controller_only(&self) -> bool {
        matches!(self, Self::ClipboardStore { .. } | Self::ClipboardLoad { .. })
    }
}

/// `Extent::Keep` on the wire.
const KEEP: u32 = u32::MAX;

fn extent_to_wire(extent: Extent) -> u32 {
    match extent {
        Extent::Keep => KEEP,
        Extent::Largest => 0,
        Extent::Size(n) => n,
    }
}

fn extent_from_wire(n: u32) -> Extent {
    match n {
        KEEP => Extent::Keep,
        0 => Extent::Largest,
        n => Extent::Size(n),
    }
}

/// A window operation as the wire's `WindowOp{op, a, b}`: `op` is XTWINOPS' number (1 deiconify, 2
/// iconify, 3 move to `a`,`b` pixels as two's complement, 5 raise, 6 lower, 8 resize to `a` rows and
/// `b` columns where `u32::MAX` keeps and 0 is the largest, 9 maximize `a` = 0 restore, 1 both, 2
/// vertical, 3 horizontal, 10 full screen `a` = 0 exit, 1 enter, 2 toggle).
pub fn window_op_to_wire(op: WindowOp) -> (u8, u32, u32) {
    match op {
        WindowOp::Deiconify => (1, 0, 0),
        WindowOp::Iconify => (2, 0, 0),
        WindowOp::Move { x, y } => (3, x as u32, y as u32),
        WindowOp::Raise => (5, 0, 0),
        WindowOp::Lower => (6, 0, 0),
        WindowOp::ResizeCells { rows, cols } => (8, extent_to_wire(rows), extent_to_wire(cols)),
        WindowOp::Maximize(how) => (
            9,
            match how {
                Maximize::Restore => 0,
                Maximize::Both => 1,
                Maximize::Vertical => 2,
                Maximize::Horizontal => 3,
            },
            0,
        ),
        WindowOp::Fullscreen(how) => (
            10,
            match how {
                Fullscreen::Exit => 0,
                Fullscreen::Enter => 1,
                Fullscreen::Toggle => 2,
            },
            0,
        ),
    }
}

/// The inverse of [`window_op_to_wire`]; `None` for a number this version does not know.
pub fn window_op_from_wire(op: u8, a: u32, b: u32) -> Option<WindowOp> {
    Some(match op {
        1 => WindowOp::Deiconify,
        2 => WindowOp::Iconify,
        3 => WindowOp::Move { x: a as i32, y: b as i32 },
        5 => WindowOp::Raise,
        6 => WindowOp::Lower,
        8 => WindowOp::ResizeCells { rows: extent_from_wire(a), cols: extent_from_wire(b) },
        9 => WindowOp::Maximize(match a {
            0 => Maximize::Restore,
            1 => Maximize::Both,
            2 => Maximize::Vertical,
            3 => Maximize::Horizontal,
            _ => return None,
        }),
        10 => WindowOp::Fullscreen(match a {
            0 => Fullscreen::Exit,
            1 => Fullscreen::Enter,
            2 => Fullscreen::Toggle,
            _ => return None,
        }),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_operations_survive_the_wire() {
        let ops = [
            WindowOp::Deiconify,
            WindowOp::Iconify,
            WindowOp::Move { x: -20, y: 300 },
            WindowOp::Raise,
            WindowOp::Lower,
            WindowOp::ResizeCells { rows: Extent::Keep, cols: Extent::Size(132) },
            WindowOp::ResizeCells { rows: Extent::Largest, cols: Extent::Keep },
            WindowOp::Maximize(Maximize::Horizontal),
            WindowOp::Fullscreen(Fullscreen::Toggle),
        ];
        for op in ops {
            let (code, a, b) = window_op_to_wire(op);
            assert_eq!(window_op_from_wire(code, a, b), Some(op), "{op:?}");
        }
        assert_eq!(window_op_from_wire(99, 0, 0), None);
        assert_eq!(window_op_from_wire(9, 7, 0), None);
    }

    #[test]
    fn an_event_maps_to_the_wire_at_its_offset() {
        let event = HostEvent::Title("build".into());
        let wire = event.to_wire(42);
        assert_eq!(wire.offset, 42);
        assert_eq!(wire.kind, wire::EventKind::Title("build"));
        assert!(HostEvent::ClipboardLoad { ticket: 1, selection: b'c' }.controller_only());
        assert!(!HostEvent::Bell.controller_only());
    }
}

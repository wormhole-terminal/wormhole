//! Where a window is, as Swift hands it over and reads it back: core's `view::layout::frame` rules
//! (reachable on the displays that are attached, stepped from the window a new one opens from) over plain
//! records. The space is the shell's own, y downwards with the primary display first.

use wormhole_core::view::layout::frame::{DisplayArea, FrameMode, WindowFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FrameKind {
    Windowed,
    Maximized,
    Fullscreen,
}

/// A window's place as saved with it.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct FrameRecord {
    pub kind: FrameKind,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub display: Option<u64>,
    /// It was in the Dock; the rectangle is where it comes back to.
    pub minimised: bool,
}

/// A display's usable area, menu bar and Dock taken off.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct DisplayRecord {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl From<FrameRecord> for WindowFrame {
    fn from(record: FrameRecord) -> Self {
        Self {
            mode: match record.kind {
                FrameKind::Windowed => FrameMode::Windowed,
                FrameKind::Maximized => FrameMode::Maximized,
                FrameKind::Fullscreen => FrameMode::Fullscreen,
            },
            x: record.x,
            y: record.y,
            width: record.width,
            height: record.height,
            display: record.display,
            minimised: record.minimised,
        }
    }
}

impl From<WindowFrame> for FrameRecord {
    fn from(frame: WindowFrame) -> Self {
        Self {
            kind: match frame.mode {
                FrameMode::Windowed => FrameKind::Windowed,
                FrameMode::Maximized => FrameKind::Maximized,
                FrameMode::Fullscreen => FrameKind::Fullscreen,
            },
            x: frame.x,
            y: frame.y,
            width: frame.width,
            height: frame.height,
            display: frame.display,
            minimised: frame.minimised,
        }
    }
}

impl From<DisplayRecord> for DisplayArea {
    fn from(d: DisplayRecord) -> Self {
        Self { id: d.id, x: d.x, y: d.y, width: d.width, height: d.height }
    }
}

/// Where to open a window saved as `frame` with these displays attached (the primary first).
#[uniffi::export]
pub fn fit_frame(frame: FrameRecord, displays: Vec<DisplayRecord>) -> FrameRecord {
    let displays: Vec<DisplayArea> = displays.into_iter().map(Into::into).collect();
    WindowFrame::from(frame).fitted(&displays).into()
}

/// A window's smallest content size.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct WindowMinSize {
    pub width: f32,
    pub height: f32,
}

/// The smallest window, the GPUI app's too: the sidebar beside a usable terminal, 400 pt tall.
#[uniffi::export]
pub fn window_min_size() -> WindowMinSize {
    use wormhole_core::view::layout::{WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH};
    WindowMinSize { width: WINDOW_MIN_WIDTH, height: WINDOW_MIN_HEIGHT }
}

/// How a terminal window opens and closes (core `view::motion::window_animation::Behaviour`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WindowAnimationKind {
    /// AppKit's own, as for a window nobody asked anything of.
    Standard,
    /// The document window's scale and fade.
    Document,
    /// None: a cut.
    Cut,
}

/// The animation a terminal window is given: the system's document one with the Motion feature on, none under Reduce
/// Motion, the default for a test instance (background) and for a torn-off window, which grows from its chip.
#[uniffi::export]
pub fn window_animation(
    motion: bool,
    reduce_motion: bool,
    background: bool,
    grows: bool,
) -> WindowAnimationKind {
    use wormhole_core::view::motion::window_animation::{Behaviour, Facts, behaviour};
    match behaviour(Facts { motion_feature: motion, reduce_motion, background, grows, platform: true }) {
        Behaviour::Default => WindowAnimationKind::Standard,
        Behaviour::System => WindowAnimationKind::Document,
        Behaviour::Cut => WindowAnimationKind::Cut,
    }
}

/// Where a window opened from `from` goes on `display`: a step down and to the right, or the corner.
#[uniffi::export]
pub fn cascade_frame(from: FrameRecord, display: DisplayRecord) -> FrameRecord {
    WindowFrame::from(from).cascaded(&display.into()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: DisplayRecord = DisplayRecord { id: 1, x: 0.0, y: 25.0, width: 1440.0, height: 875.0 };

    fn frame(x: f32) -> FrameRecord {
        FrameRecord {
            kind: FrameKind::Windowed,
            x,
            y: 100.0,
            width: 900.0,
            height: 600.0,
            display: None,
            minimised: false,
        }
    }

    #[test]
    fn the_records_go_through_core_s_rules() {
        assert_eq!(fit_frame(frame(100.0), vec![MAIN]), frame(100.0));
        let moved = fit_frame(frame(9000.0), vec![MAIN]);
        assert_eq!((moved.x, moved.display), (40.0, Some(1)));
        let next = cascade_frame(frame(100.0), MAIN);
        assert_eq!((next.x, next.y, next.display), (126.0, 126.0, Some(1)));
    }

    /// A window saved full screen or zoomed comes back as such, also when its display is gone and one in the Dock stays there.
    #[test]
    fn a_saved_full_screen_or_zoomed_window_keeps_its_kind_when_its_display_is_gone() {
        for kind in [FrameKind::Fullscreen, FrameKind::Maximized] {
            let lost = FrameRecord { kind, display: Some(2), minimised: true, ..frame(9000.0) };
            let fitted = fit_frame(lost, vec![MAIN]);
            assert_eq!((fitted.kind, fitted.minimised), (kind, true));
        }
    }
}

//! A terminal's status tile and the glyph its state is drawn with: its agent's logo or the terminal
//! symbol, and its state round it; the bell is a badge on a terminal with nothing louder to say.

use gpui_kit::*;
use wormhole_core::view::sidebar::RowState;

use super::super::facts::RowInfo;
use super::Frame;
use crate::style::{self, StatusGlyph, StatusShift, StatusTile, TileFace};

/// A terminal's status tile: its agent's logo (when the rows show it) or the terminal symbol, and
/// its state round it; the bell is a badge on a terminal that has nothing louder to say.
pub(in crate::sidebar) fn tile_for(
    info: &RowInfo,
    frame: &Frame,
    selected: bool,
    size: f32,
    cx: &App,
) -> AnyElement {
    tile_shifting(info, frame, selected, size, None, cx)
}

/// [`tile_for`] while the terminal's status changes to the one it shows, as `shift` plays it.
pub(in crate::sidebar) fn tile_shifting(
    info: &RowInfo,
    frame: &Frame,
    selected: bool,
    size: f32,
    shift: Option<StatusShift>,
    cx: &App,
) -> AnyElement {
    let (face, state) = tile_parts(info, frame);
    let id = ElementId::Name(SharedString::from(format!("tile-{}-{}", info.id, size as u32)));
    let surface = if selected { frame.ui.selected_row } else { frame.ui.panel };
    style::status_tile_shifting(
        id,
        frame.ui,
        StatusTile { face, glyph: state, selected, mini: false },
        size,
        surface,
        shift,
        cx,
    )
}

/// What a terminal's tile shows: the face and the state, for a tile drawn later (the tab bar's
/// switcher builds its content without the workspace).
pub(in crate::sidebar) fn tile_parts(info: &RowInfo, frame: &Frame) -> (TileFace, StatusGlyph) {
    let face = match info.agent {
        Some(agent) => TileFace::Agent(agent),
        None => TileFace::Terminal,
    };
    let state = match glyph_for(info) {
        _ if !frame.show_dot => StatusGlyph::Idle,
        StatusGlyph::Shell | StatusGlyph::Idle | StatusGlyph::Terminal { unread: false } if info.bell => {
            StatusGlyph::Bell
        }
        state => state,
    };
    (face, state)
}

/// The glyph a terminal's state is drawn with.
pub(in crate::sidebar) fn glyph_for(info: &RowInfo) -> StatusGlyph {
    match (info.told(), info.state) {
        (_, RowState::Exited) => StatusGlyph::Shell,
        (true, RowState::Working) => StatusGlyph::Working,
        (true, RowState::NeedsYou) => StatusGlyph::NeedsYou,
        (true, RowState::Finished) => StatusGlyph::Done,
        (_, RowState::Failed) => StatusGlyph::Failed,
        (true, RowState::Idle) => StatusGlyph::Idle,
        (false, _) if info.command.is_some() => StatusGlyph::Terminal { unread: false },
        // The last command failed and nothing runs now: the row keeps saying so until the next one
        // (seen or not), in place of the quiet terminal and its output.
        (false, _) if info.failure.is_some() => StatusGlyph::Failed,
        (false, _) if info.unread => StatusGlyph::Terminal { unread: true },
        (false, _) => StatusGlyph::Shell,
    }
}

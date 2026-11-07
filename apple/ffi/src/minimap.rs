//! The minimap's lines (`wormhole_term::minimap`): the history and the screen as runs of one colour, read
//! under the engine's lock only when the output or the view changed, and a scroll that puts the view at
//! a given depth into the history. Swift keeps the lines it has and asks for them again when `stamp` moves.

use std::sync::{Mutex, PoisonError};

use wormhole_term::minimap::{MinimapGeometry, MinimapSnapshot};
use wormhole_term::types::Scroll;

use crate::frame::{pack_color, pack_flags};
use crate::terminal::Terminal;

/// What the terminal keeps between minimap calls.
#[derive(Default)]
pub(crate) struct Minimap {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    snapshot: MinimapSnapshot,
    /// What `stamp` was last bumped for: the output revision, the geometry and the columns.
    read_for: Option<(u64, MinimapGeometry, usize)>,
    stamp: u64,
}

/// Where the view is among all the lines, for a minimap with room for `fits` of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct MinimapGeometryRecord {
    /// History and screen together.
    pub total: u32,
    pub screen: u32,
    pub fits: u32,
    /// The first line on screen.
    pub view_top: u32,
    pub columns: u32,
    /// Changes whenever `minimap_lines` would return something different.
    pub stamp: u64,
}

/// Columns `start..end` of one line drawn as one block in the colours of its first cell: the text colour
/// for text, the background for coloured blanks. Colours and flags are the frame's (`frame.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct MinimapRunRecord {
    pub start: u32,
    pub end: u32,
    pub text: bool,
    pub scalar: u32,
    pub fg: u32,
    pub bg: u32,
    pub flags: u32,
}

#[uniffi::export]
impl Terminal {
    /// Bring the minimap's copy up to date for a minimap `fits` lines tall; cheap when nothing changed.
    pub fn minimap_geometry(&self, fits: u32) -> MinimapGeometryRecord {
        let mut state = self.minimap.state.lock().unwrap_or_else(PoisonError::into_inner);
        let revision = self.with_pty(|pty| pty.output_revision()).unwrap_or(0);
        self.with_pty(|pty| pty.minimap(&mut state.snapshot, fits as usize));
        let g = state.snapshot.geometry;
        let key = (revision, g, state.snapshot.cols);
        if state.read_for != Some(key) {
            state.read_for = Some(key);
            state.stamp += 1;
        }
        MinimapGeometryRecord {
            total: g.total as u32,
            screen: g.screen as u32,
            fits: g.fits as u32,
            view_top: g.view_top as u32,
            columns: state.snapshot.cols as u32,
            stamp: state.stamp,
        }
    }

    /// The runs of each line shown, from the geometry's first line on.
    pub fn minimap_lines(&self) -> Vec<Vec<MinimapRunRecord>> {
        let state = self.minimap.state.lock().unwrap_or_else(PoisonError::into_inner);
        state
            .snapshot
            .lines
            .iter()
            .map(|runs| {
                runs.iter()
                    .map(|run| MinimapRunRecord {
                        start: run.start as u32,
                        end: run.end as u32,
                        text: run.text,
                        scalar: u32::from(run.cell.c),
                        fg: pack_color(run.cell.fg),
                        bg: pack_color(run.cell.bg),
                        flags: pack_flags(&run.cell),
                    })
                    .collect()
            })
            .collect()
    }

    /// Scroll the view so it is `offset` lines into the history (0 the live screen).
    pub fn scroll_to_display_offset(&self, offset: u32) {
        self.with_pty(|pty| {
            let now = pty.display_offset() as i32;
            pty.scroll(Scroll::Delta(offset as i32 - now));
        });
    }
}

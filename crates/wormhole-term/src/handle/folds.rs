//! The folds of a terminal's history (`crate::folds`), asked for by the app.

use std::sync::Arc;

use wormhole_vt::Line;

use crate::folds::{FoldOwner, Refused, Request};
use crate::{BlockState, Pty, blocks, sys};

impl Pty {
    /// Fold the output of the finished command block `id` into one marker row. Only what the rules in
    /// `folds` allow is folded (the live screen is never); the result is the fold's id.
    pub fn fold_block_output(&self, id: u64) -> Result<u64, Refused> {
        self.fold_block(id, None, (0, 0))
    }

    /// A Probe's fold of block `id`'s output, less the first `keep.0` and the last `keep.1` lines, which
    /// stay shown. Refused while a program reads a secret, besides what [`fold_block_output`] refuses.
    ///
    /// [`fold_block_output`]: Self::fold_block_output
    pub fn fold_block_output_by(&self, id: u64, owner: FoldOwner, keep: (u32, u32)) -> Result<u64, Refused> {
        if self.secure_input() {
            return Err(Refused::Secure);
        }
        self.fold_block(id, Some(Arc::new(owner)), keep)
    }

    fn fold_block(&self, id: u64, owner: Option<Arc<FoldOwner>>, keep: (u32, u32)) -> Result<u64, Refused> {
        let mut term = self.term.lock();
        let block = blocks::block(&term, id, &self.shared.commands()).ok_or(Refused::Unfinished)?;
        let (Some((first, last)), BlockState::Finished { .. }) = (block.output_lines(), block.state) else {
            return Err(Refused::Unfinished);
        };
        let (first, last) = (first + keep.0 as i32, last - keep.1 as i32);
        term.fold_add(first, last, Request { finished: true, failed: block.state.failed(), owner })
    }

    /// Whether any fold exists.
    pub fn has_folds(&self) -> bool {
        self.term.lock().has_folds()
    }

    /// Open every fold (Show Hidden Lines); whether any was closed.
    pub fn fold_show_all(&self) -> bool {
        self.term.lock().fold_show_all()
    }

    /// Open the folds in the last two screens for good (a bell, or a program's report that it needs input).
    pub fn fold_expand_recent(&self) {
        self.term.lock().fold_expand_recent();
    }

    /// Keep only the folds of the Probes in `probes` (the ones on that may fold): the others' go at once.
    pub fn fold_retain_probes(&self, probes: &[String]) {
        self.term.lock().fold_retain_probes(probes);
    }

    /// The person's "never hide lines": while on, the view shows every line and nothing is folded.
    /// Whether this changed it.
    pub fn set_folds_off(&self, off: bool) -> bool {
        self.term.lock().fold_set_off(off)
    }

    /// Whether a program is reading a secret just now: echo is off on the terminal (a password prompt).
    /// `false` where the system cannot say. A shell editing its own line has echo off as well (zsh's ZLE,
    /// readline), and that is no secret: only a program in front of the shell reads one, so while the shell
    /// holds the terminal nothing is secret, or no fold would ever be made at a prompt.
    pub fn secure_input(&self) -> bool {
        if self.program_in_foreground() == Some(false) {
            return false;
        }
        self.pid().and_then(sys::process::echo_off).unwrap_or(false)
    }

    /// Look at echo again and tell the folds (a secret being typed opens the recent ones and refuses new
    /// ones, as a terminal that needs the person does). Whether the answer changed.
    pub fn refresh_fold_secure(&self) -> bool {
        let secure = self.secure_input();
        self.term.lock().fold_set_secure(secure)
    }

    /// Open or close fold `id`; whether it exists.
    pub fn fold_toggle(&self, id: u64) -> bool {
        self.term.lock().fold_toggle(id)
    }

    /// Open the fold hiding `line`, if there is one (find stepping to a match in it); whether one opened.
    pub fn fold_expand_at(&self, line: Line) -> bool {
        self.term.lock().fold_expand_at(line)
    }

    /// Say whether the terminal needs the person (see `folds`): it opens the recent folds and refuses
    /// new ones while it does.
    pub fn set_fold_needs_you(&self, needs_you: bool) {
        self.term.lock().fold_set_needs_you(needs_you);
    }
}

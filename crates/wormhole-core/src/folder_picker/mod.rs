//! The folder picker (⌘J): the generic picker (`view::picker`) with the folder providers plugged in. Type a path
//! to walk the folders below the terminal's (`src/`, `../`, `~/`, `/usr/`), or part of a name, or part of
//! several (`pr/te` for `Projects/terminal`); pick one and its path is put at the cursor (the picker never
//! changes directory).
//!
//! - `typed`: what the typed text means (where it starts, its steps) and the way up from it.
//! - `walk`: finding the folders it names, forgivingly, through up to four levels, under hard bounds.
//! - `collate`: the order `ls` lists names in; `insert`: the shell-ready text a pick makes.
//! - `recent`: the folders terminals were in lately, by frecency (pure; the app keeps it in the store).
//! - `source`: the providers ([`Subfolders`], [`Recent`]) and decorators ([`Chevrons`], [`Branches`]), the
//!   only parts that read the disk.
//!
//! It sits above `git` (a repository's branch is read by `git::branch`) and below the app; `view::picker`
//! knows none of it.

mod collate;
mod insert;
mod recent;
mod source;
mod typed;
mod walk;

use std::sync::{Arc, Mutex};

pub use collate::collate;
pub use insert::{inserted, shell_form};
pub use recent::{CAP as RECENT_CAP, RecentFolders, keep_out};
pub use source::{Branches, Chevrons, KIND, Recent, Subfolders, display, path_crumbs};
pub use typed::{Typed, parse, tidy, up};
pub use walk::{Found, Hit, Limits, walk};

use crate::view::picker::{Decorator, Provider};

/// What the folder picker plugs into the generic one: each provider is asked on its own, so the folders of
/// the current folder never wait for a recent folder on a share that does not answer.
pub struct Parts {
    pub providers: Vec<Arc<dyn Provider>>,
    pub decorators: Vec<Arc<dyn Decorator>>,
}

/// The folder picker's providers and decorators: the subfolders, then the recent folders (under
/// `recent_title`, shown when nothing is typed), and the chevron and the branch badge.
pub fn parts(recents: Arc<Mutex<RecentFolders>>, recent_title: &str) -> Parts {
    Parts {
        providers: vec![Arc::new(Subfolders::default()), Arc::new(Recent::new(recents, recent_title))],
        decorators: vec![Arc::new(Chevrons), Arc::new(Branches)],
    }
}

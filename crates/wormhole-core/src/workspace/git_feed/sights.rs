//! How each terminal is seen, which sets how often its repository is read.

use std::path::PathBuf;

use super::{GitFeed, Sight};

impl GitFeed {
    pub fn known(&self) -> impl Iterator<Item = i64> + '_ {
        self.terminals.keys().copied()
    }

    pub fn set_seen(&mut self, seen: bool) {
        self.seen = seen;
    }

    /// How each terminal is seen. In a window that can be seen, the terminal on screen in the window in
    /// front is looked at and the others are behind; in one that cannot, the repositories something
    /// works in are read as when behind (their pictures say who changed what, which is not known
    /// afterwards), and the rest only now and then.
    pub fn set_sights(&mut self, active: bool, terminals: impl Iterator<Item = (i64, bool, bool)>) {
        let terminals: Vec<_> = terminals.collect();
        let working: Vec<PathBuf> = if self.seen {
            Vec::new()
        } else {
            terminals.iter().filter(|(_, _, works)| *works).filter_map(|(id, ..)| self.root_of(*id)).collect()
        };
        for (id, on_screen, _) in terminals {
            let sight = if self.seen {
                if active && on_screen { Sight::Front } else { Sight::Behind }
            } else if self.root_of(id).is_some_and(|root| working.contains(&root)) {
                Sight::Behind
            } else {
                Sight::Unseen
            };
            self.sight(id, sight);
        }
    }

    fn root_of(&self, id: i64) -> Option<PathBuf> {
        let state = self.terminals.get(&id)?;
        match state.git.place(state.cwd.as_deref()) {
            crate::git::attribution::Place::In(root) => Some(root.to_path_buf()),
            _ => None,
        }
    }
}

//! Who a user is, in the terms the system compares: the effective user id on Unix. Every "is this
//! ours?" check (a socket's peer, a folder's owner, a name that embeds the user) compares two of
//! these, so they are never a real id against an effective one.

use std::fmt;
use std::fs::Metadata;

use super::unix::user as os;

/// A user of this machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UserId(u32);

impl UserId {
    /// The user this process acts as (its effective uid: the one the system checks files and
    /// folders against, and the one the files and sockets it makes belong to).
    pub fn current() -> Self {
        Self(os::effective_uid())
    }

    /// The owner of a file or folder.
    pub fn owner_of(meta: &Metadata) -> Self {
        Self(os::owner(meta))
    }

    /// A user the system reported as its raw id (a socket's peer).
    pub(super) fn from_raw(id: u32) -> Self {
        Self(id)
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_made_here_is_ours() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let meta = std::fs::metadata(dir.path()).expect("exists");
        assert_eq!(UserId::owner_of(&meta), UserId::current());
    }

    #[test]
    fn a_user_prints_as_its_number() {
        assert_eq!(UserId::from_raw(501).to_string(), "501");
    }
}

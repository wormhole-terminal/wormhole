//! Where a session host listens. The host is the only one that decides it, by this rule, and writes
//! what it chose into its meta file (`socket=`); everyone else reads the meta, never the rule.
//!
//! The socket belongs in `<hosts dir>/<id>.sock`, next to the meta, in the `0700` directory. A
//! Unix socket's path must fit `sun_path`, which a long home folder can break; the socket then goes
//! where [`sys::socket::socket_in`] puts it, named `wormhole-host-<uid>-<id>.sock`. A socket outside
//! the hosts directory is in a directory others may write to, so a client checks the peer it
//! reaches there (its uid) before it believes it is the host the meta names.

use std::path::{Path, PathBuf};

use crate::sys::socket::socket_in;
use crate::sys::user::UserId;

/// Where the host with `id` listens, given its hosts directory (see the module docs).
pub fn socket_path(hosts_dir: &Path, id: &str) -> PathBuf {
    socket_path_for(hosts_dir, id, UserId::current())
}

/// `<hosts dir>/<id>.meta`.
pub fn meta_path(hosts_dir: &Path, id: &str) -> PathBuf {
    super::meta::path_in(hosts_dir, id)
}

/// [`socket_path`] for the user `uid` (a test names one).
fn socket_path_for(hosts_dir: &Path, id: &str, uid: impl std::fmt::Display) -> PathBuf {
    socket_in(&hosts_dir.join(format!("{id}.sock")), &format!("wormhole-host-{uid}-{id}.sock"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "abcdefghij12";

    #[test]
    fn a_short_hosts_dir_holds_the_socket() {
        let dir = Path::new("/Users/me/Library/Application Support/Wormhole/hosts");
        assert_eq!(socket_path_for(dir, ID, 501), dir.join("abcdefghij12.sock"));
    }

    #[test]
    fn a_long_one_names_the_socket_for_the_user_and_the_id() {
        let long = PathBuf::from(format!("/Users/{}/hosts", "x".repeat(90)));
        let path = socket_path_for(&long, ID, 501);
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("wormhole-host-501-abcdefghij12.sock")
        );
        assert!(!path.starts_with(&long), "{path:?}");
    }
}

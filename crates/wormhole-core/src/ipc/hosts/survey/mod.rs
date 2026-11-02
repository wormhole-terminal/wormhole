//! Looking at the hosts of a folder: the scan, and for each host that runs what the system and the
//! host itself can say (`Observed`). All the I/O of the Session Hosts report is here, so
//! [`report`](super::report) stays a function of its inputs.
//!
//! It takes at most `wait` per host and asks them all at once, so a stuck host costs one wait, not
//! one per host. Nothing here changes a host or a file.

use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::time::Duration;

use super::identity::{Identity, identify};
use super::probe::probe;
use super::report::Observed;
use super::{HostMeta, Scan, scan};
use crate::platform::process;

/// The scan and what was seen of each host in it.
#[derive(Debug, Default)]
pub struct Survey {
    pub scan: Scan,
    pub observed: HashMap<String, Observed>,
}

/// Scan `hosts_dir`, then look at every host whose process runs. A live host is asked over its
/// socket too; a stale one that still has a process (its socket is missing) is only read from the
/// system, since there is nothing to ask.
pub fn survey(hosts_dir: &Path, wait: Duration) -> io::Result<Survey> {
    let scan = scan(hosts_dir)?;
    let mut observed = HashMap::new();
    let live = scan.live.iter().map(|meta| (meta, true));
    let stale = scan.stale.iter().filter_map(|s| s.meta.as_ref()).map(|meta| (meta, false));
    let hosts: Vec<_> = live.chain(stale).collect();
    let seen = super::chunked::map_chunked(&hosts, || None, |&(meta, ask)| look(meta, ask, wait));
    observed.extend(hosts.iter().zip(seen).filter_map(|(&(meta, _), seen)| Some((meta.id.clone(), seen?))));
    Ok(Survey { scan, observed })
}

/// What is known of one host's process, and its answer when `ask`ed. Only a process that is the
/// host (its name, its user and its start fit the meta) is read: a pid that something else has
/// now must not put that program's folder or memory in front of a person as the host's.
fn look(meta: &HostMeta, ask: bool, wait: Duration) -> Option<Observed> {
    if identify(meta) != Identity::Yes {
        return None;
    }
    let (program, cwd) = match meta.child_pid {
        Some(shell) => (foreground_program(shell), process::cwd(shell)),
        None => (None, None),
    };
    Some(Observed {
        probed: ask.then(|| probe(meta, wait)),
        rss: process::resident_bytes(meta.pid),
        program,
        cwd,
    })
}

/// The name of the program that owns the terminal of `shell` right now (the shell itself when
/// nothing else does).
fn foreground_program(shell: u32) -> Option<String> {
    let leader = process::foreground_group(shell).unwrap_or(shell);
    let path = process::executable(leader).or_else(|| process::executable(shell))?;
    path.file_name().map(|name| name.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests;

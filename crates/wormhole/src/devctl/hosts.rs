//! `hosts` and `host-kill`: the session hosts of this instance's data folder, the way
//! `Wormhole sessions list` and `sessions end` see them (the same code, so what a check reads
//! here is what a user's command line prints).

use anyhow::{Result, anyhow};
use wormhole_core::hosts::hosts_dir;
use wormhole_core::ipc::control::sessions;

use super::Ctx;

/// `hosts`: every host with its pid, shell pid, start, version and whether it is live or stale.
pub(super) fn hosts(_: &mut Ctx, _: &str) -> Result<String> {
    run(&["list"])
}

/// `host-kill <id>`: end that host as `sessions end <id>` does (`Kill` over its socket, SIGTERM
/// when that fails); prints what happened.
pub(super) fn host_kill(_: &mut Ctx, arg: &str) -> Result<String> {
    run(&["end", arg.trim()])
}

fn run(args: &[&str]) -> Result<String> {
    let args: Vec<String> = args.iter().map(ToString::to_string).collect();
    let (code, out, err) = sessions::run_in(&args, &hosts_dir(&wormhole_core::store::data_dir()));
    if code == 0 { Ok(out) } else { Err(anyhow!("{}", err.trim())) }
}

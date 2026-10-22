//! `hosts` and `host-kill`: the session hosts of this instance's data folder, the way
//! `Wormhole sessions list` and `sessions end` see them (the same code, so what a check reads
//! here is what a user's command line prints). `detach`, `reattach` and `quit-keep`: what the
//! app does with its hosted terminals as it quits and launches, one window at a time.

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

/// `detach`: let go of this window's hosted terminals, leaving the views; prints how many.
pub(super) fn detach(ctx: &mut Ctx, _: &str) -> Result<String> {
    Ok(format!("detached {}", ctx.workspace.detach_hosts(ctx.cx)))
}

/// `reattach`: attach the terminals let go of again; prints their ids, one a line.
pub(super) fn reattach(ctx: &mut Ctx, _: &str) -> Result<String> {
    let ids = ctx.workspace.reattach_hosts(ctx.cx);
    Ok(if ids.is_empty() { "nothing to reattach".into() } else { ids.join("\n") })
}

/// `quit-keep`: quit as ⌘Q does (the quit lets go of every host before the windows go).
pub(super) fn quit_keep(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.workspace.quit(ctx.cx);
    Ok(String::new())
}

fn run(args: &[&str]) -> Result<String> {
    let args: Vec<String> = args.iter().map(ToString::to_string).collect();
    let (code, out, err) = sessions::run_in(&args, &hosts_dir(&wormhole_core::paths::data_dir()));
    if code == 0 { Ok(out) } else { Err(anyhow!("{}", err.trim())) }
}

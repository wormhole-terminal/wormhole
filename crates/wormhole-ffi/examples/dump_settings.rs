//! Writes `wormhole_ffi::settings_dump::json()` to the path given: the Swift UI's `settings.json`.
//! `cargo run -p wormhole-ffi --example dump_settings -- <out.json>` (`scripts/dump-settings.sh`).

#![allow(clippy::print_stdout, reason = "a command line tool")]

#[allow(clippy::disallowed_methods, reason = "a developer tool writing one file the user named")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args().nth(1).ok_or("usage: dump_settings <out.json>")?;
    std::fs::write(&out, wormhole_ffi::settings_dump::json())?;
    println!("wrote {out}");
    Ok(())
}

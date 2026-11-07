//! Writes `wormhole_ffi::theme_dump::json()` to the path given: the Swift UI's `themes.json`.
//! `cargo run -p wormhole-ffi --example dump_themes -- <out.json>` (`scripts/dump-themes.sh`).

#![allow(clippy::print_stdout, reason = "a command line tool")]

#[allow(clippy::disallowed_methods, reason = "a developer tool writing one file the user named")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args().nth(1).ok_or("usage: dump_themes <out.json>")?;
    std::fs::write(&out, wormhole_ffi::theme_dump::json())?;
    println!("wrote {out}");
    Ok(())
}

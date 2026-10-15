//! Links libghostty-vt when `fetch-ghostty.sh` has built it, and compiles the C shim that gives
//! Rust a flat face for it. Without the library the Ghostty engine is left out (cfg `has_ghostty`
//! unset) and everything else still builds and runs: a contributor without Zig can work on the rest.

use std::env;
use std::path::PathBuf;

fn main() {
    versions_from_lockfile();
    println!("cargo:rustc-check-cfg=cfg(has_ghostty)");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=ghostty.pin");
    println!("cargo:rerun-if-changed=shim/ghostty_shim.c");
    println!("cargo:rerun-if-changed=shim/ghostty_shim.h");
    println!("cargo:rerun-if-env-changed=WORMHOLE_VT_COMPARE_CACHE");
    println!("cargo:rerun-if-env-changed=GHOSTTY_VT_DIR");

    let Some(dir) = ghostty_dir() else {
        println!(
            "cargo:warning=libghostty-vt not found: the ghostty engine is left out. \
             Run tools/vt-compare/fetch-ghostty.sh (needs git and network; it fetches Zig if needed)."
        );
        return;
    };
    let lib = dir.join("lib");
    println!("cargo:rerun-if-changed={}", lib.join("libghostty-vt.a").display());

    cc::Build::new()
        .file("shim/ghostty_shim.c")
        .include(dir.join("include"))
        .include("shim")
        .warnings(true)
        .compile("ghostty_shim");

    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=static=ghostty-vt");
    println!("cargo:rustc-cfg=has_ghostty");
}

/// The versions the report names each engine by, from `Cargo.lock` (`VT100_VERSION`,
/// `ALACRITTY_TERMINAL_VERSION`, `WEZTERM_TERM_REV`) and `ghostty.pin` (`GHOSTTY_REV`), so they are
/// what was built and not what someone remembered to write down.
fn versions_from_lockfile() {
    println!("cargo:rerun-if-changed=Cargo.lock");
    let lock = std::fs::read_to_string("Cargo.lock").unwrap_or_default();
    let (mut name, mut version, mut source) = (String::new(), String::new(), String::new());
    let (mut vt100, mut alacritty, mut wezterm) = (String::new(), String::new(), String::new());
    // A package ends at the blank line after it (and the file at its end).
    for line in lock.lines().chain(std::iter::once("")) {
        if line.is_empty() {
            match name.as_str() {
                "vt100" => vt100 = version.clone(),
                "alacritty_terminal" => alacritty = version.clone(),
                "wezterm-term" => {
                    // git+https://github.com/wezterm/wezterm?rev=<rev>#<commit>
                    wezterm = source.rsplit('#').next().unwrap_or_default().chars().take(8).collect();
                }
                _ => {}
            }
            (name, version, source) = (String::new(), String::new(), String::new());
        } else if let Some(v) = line.strip_prefix("name = ") {
            name = v.trim_matches('"').to_string();
        } else if let Some(v) = line.strip_prefix("version = ") {
            version = v.trim_matches('"').to_string();
        } else if let Some(v) = line.strip_prefix("source = ") {
            source = v.trim_matches('"').to_string();
        }
    }
    let pin = std::fs::read_to_string("ghostty.pin").unwrap_or_default();
    let ghostty = pin
        .lines()
        .find_map(|l| l.strip_prefix("rev="))
        .unwrap_or_default()
        .chars()
        .take(8)
        .collect::<String>();
    for (key, value) in [
        ("VT100_VERSION", vt100),
        ("ALACRITTY_TERMINAL_VERSION", alacritty),
        ("WEZTERM_TERM_REV", wezterm),
        ("GHOSTTY_REV", ghostty),
    ] {
        println!("cargo:rustc-env={key}={}", if value.is_empty() { "unknown".to_string() } else { value });
    }
}

/// The folder with `lib/libghostty-vt.a` and `include/ghostty/vt.h`: `$GHOSTTY_VT_DIR`, or where
/// fetch-ghostty.sh puts the pinned build.
fn ghostty_dir() -> Option<PathBuf> {
    let complete = |dir: &PathBuf| {
        dir.join("lib/libghostty-vt.a").is_file() && dir.join("include/ghostty/vt.h").is_file()
    };
    if let Some(dir) = env::var_os("GHOSTTY_VT_DIR").map(PathBuf::from) {
        return complete(&dir).then_some(dir);
    }
    let pin = std::fs::read_to_string("ghostty.pin").ok()?;
    let rev = pin.lines().find_map(|l| l.strip_prefix("rev="))?.trim().to_string();
    let cache = match env::var_os("WORMHOLE_VT_COMPARE_CACHE") {
        Some(dir) => PathBuf::from(dir),
        None => {
            let home = PathBuf::from(env::var_os("HOME")?);
            if cfg!(target_os = "macos") {
                home.join("Library/Caches/wormhole/vt-compare")
            } else {
                match env::var_os("XDG_CACHE_HOME") {
                    Some(xdg) => PathBuf::from(xdg).join("wormhole/vt-compare"),
                    None => home.join(".cache/wormhole/vt-compare"),
                }
            }
        }
    };
    let dir = cache.join("ghostty-vt").join(rev.get(..12)?);
    complete(&dir).then_some(dir)
}

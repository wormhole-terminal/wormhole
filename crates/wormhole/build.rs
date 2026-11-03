//! Builds the native Settings window (`macos/settings/`, a Swift package) into a static library and links it,
//! with the Swift runtime it needs. Only for macOS targets; elsewhere there is nothing to do. The Swift sources are
//! the only input, so cargo runs this again exactly when they change.

// A build script cannot use `wormhole_term::sys::process::command` (it is a dependency of the package this builds).
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=macos/settings/Package.swift");
    println!("cargo:rerun-if-changed=macos/settings/Sources");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let manifest =
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let arch =
        if target_arch == "aarch64" || target_arch.is_empty() { "arm64" } else { target_arch.as_str() };
    let min = std::env::var("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|_| "14.0".into());
    let triple = format!("{arch}-apple-macosx{min}");
    let package = manifest.join("macos/settings");
    let scratch = out.join("swift");

    let swift = |extra: &[&str]| {
        let mut command = Command::new("swift");
        command
            .args(["build", "-c", "release", "--product", "SettingsUI", "--triple", &triple])
            .arg("--package-path")
            .arg(&package)
            .arg("--scratch-path")
            .arg(&scratch)
            .args(extra);
        command
    };
    let status = swift(&[]).status().unwrap_or_else(|err| {
        panic!("running `swift build` (is Xcode or its command line tools installed?): {err}")
    });
    assert!(status.success(), "`swift build` of macos/settings failed");
    let bin = swift(&["--show-bin-path"]).output().expect("`swift build --show-bin-path`");
    assert!(bin.status.success(), "`swift build --show-bin-path` failed");
    let bin = String::from_utf8(bin.stdout).expect("a path");
    println!("cargo:rustc-link-search=native={}", bin.trim());
    // As an argument of the final link, not `rustc-link-lib`: that is passed to the library target only, and the
    // binary and the tests are what call into Swift.
    println!("cargo:rustc-link-arg={}/libSettingsUI.a", bin.trim());

    // The static library names the Swift runtime in its objects (autolink); the linker finds its stubs in the SDK, the
    // back-deployment libraries in the toolchain, and the app finds the runtime itself in the OS at launch.
    if let Some(sdk) = run(&["xcrun", "--show-sdk-path"]) {
        println!("cargo:rustc-link-search=native={sdk}/usr/lib/swift");
    }
    let toolchain = run(&["xcrun", "--find", "swiftc"]);
    if let Some(toolchain) = toolchain.as_deref().map(Path::new).and_then(Path::parent).and_then(Path::parent)
    {
        for dir in ["lib/swift/macosx", "lib/swift-5.5/macosx", "lib/swift-6.0/macosx"] {
            let dir = toolchain.join(dir);
            if dir.is_dir() {
                println!("cargo:rustc-link-search=native={}", dir.display());
            }
        }
    }
    println!("cargo:rustc-link-search=native=/usr/lib/swift");
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
}

fn run(command: &[&str]) -> Option<String> {
    let output = Command::new(command[0]).args(&command[1..]).output().ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

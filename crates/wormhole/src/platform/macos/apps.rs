//! AppKit's and Launch Services' side of `platform::apps`: apps by bundle identifier, their icons,
//! and `open`, which asks an app to open what it is given (`-b`), starts a new one with arguments
//! (`-n --args`), reveals in Finder (`-R`) or uses the text editor (`-t`).

use std::path::{Path, PathBuf};
use std::process::Command;

use objc2::AnyThread as _;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSCompositingOperation, NSDeviceRGBColorSpace,
    NSGraphicsContext, NSWorkspace,
};
use objc2_foundation::{NSBundle, NSDictionary, NSPoint, NSRect, NSSize, NSString};
use wormhole_core::open_file::Launch;

pub fn app_path(bundle: &str) -> Option<PathBuf> {
    let url =
        NSWorkspace::sharedWorkspace().URLForApplicationWithBundleIdentifier(&NSString::from_str(bundle))?;
    url.path().map(|path| PathBuf::from(path.to_string()))
}

pub fn bundle_of(path: &Path) -> Option<String> {
    let bundle = NSBundle::bundleWithPath(&NSString::from_str(&path.to_string_lossy()))?;
    bundle.bundleIdentifier().map(|id| id.to_string())
}

/// The icon of the app with this bundle identifier as a PNG, `size` pixels square.
#[allow(unsafe_code)] // AppKit's bitmap initializer takes raw plane pointers; see the SAFETY note.
pub fn icon_png(bundle: &str, size: usize) -> Option<Vec<u8>> {
    let path = app_path(bundle)?;
    let icon = NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(&path.to_string_lossy()));
    let side = size as isize;
    // SAFETY: no planes are passed (a null pointer asks the representation to allocate its own);
    // the geometry is a plain 8-bit RGBA bitmap.
    let rep: Retained<NSBitmapImageRep> = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            side,
            side,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            0,
            0,
        )
    }?;
    let context = NSGraphicsContext::graphicsContextWithBitmapImageRep(&rep)?;
    NSGraphicsContext::saveGraphicsState_class();
    NSGraphicsContext::setCurrentContext(Some(&context));
    icon.drawInRect_fromRect_operation_fraction(
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(size as f64, size as f64)),
        NSRect::ZERO,
        NSCompositingOperation::SourceOver,
        1.0,
    );
    NSGraphicsContext::restoreGraphicsState_class();
    // SAFETY: the properties dictionary is empty, so its element type is never read.
    let data =
        unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) }?;
    Some(data.to_vec())
}

/// The process for a [`Launch`]: `open` aimed at the app, or the command line the app brings.
pub fn command(bundle: &str, launch: Launch) -> Option<Command> {
    let open = || Command::new("open");
    let in_app = |what: &dyn AsRef<std::ffi::OsStr>| {
        let mut cmd = open();
        cmd.arg("-b").arg(bundle).arg(what.as_ref());
        cmd
    };
    Some(match launch {
        Launch::Open { path } => in_app(&path),
        Launch::Url { url } => in_app(&url),
        Launch::Reveal { path } => {
            let mut cmd = open();
            cmd.arg("-R").arg(path);
            cmd
        }
        Launch::Args { args } => {
            let mut cmd = open();
            cmd.arg("-n").arg("-b").arg(bundle).arg("--args").args(args);
            cmd
        }
        Launch::Bundled { relative, arg } => {
            match app_path(bundle).map(|app| app.join(relative)).filter(|exe| exe.is_file()) {
                Some(exe) => {
                    let mut cmd = Command::new(exe);
                    cmd.arg(arg);
                    cmd
                }
                // The app doesn't have it (an older version): the file, without the line.
                None => {
                    let path = arg.rsplit_once(':').map_or(arg.as_str(), |(path, _)| path).to_string();
                    in_app(&path)
                }
            }
        }
    })
}

/// `open -t`: the text editor macOS picks for the file.
pub fn text_editor_command(path: &Path) -> Option<Command> {
    let mut cmd = Command::new("open");
    cmd.arg("-t").arg(path);
    Some(cmd)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::command;
    use wormhole_core::open_file::Launch;

    fn cmd(bundle: &str, launch: Launch) -> std::process::Command {
        command(bundle, launch).expect("macOS launches with open")
    }

    fn words(cmd: &std::process::Command) -> Vec<String> {
        std::iter::once(cmd.get_program())
            .chain(cmd.get_args())
            .map(|word| word.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn each_launch_is_aimed_at_the_chosen_app() {
        let path = PathBuf::from("/repo/a.rs");
        assert_eq!(
            words(&cmd("dev.zed.Zed", Launch::Open { path: path.clone() })),
            ["open", "-b", "dev.zed.Zed", "/repo/a.rs"]
        );
        assert_eq!(
            words(&cmd("com.microsoft.VSCode", Launch::Url { url: "vscode://file/repo/a.rs:3".into() })),
            ["open", "-b", "com.microsoft.VSCode", "vscode://file/repo/a.rs:3"]
        );
        assert_eq!(words(&cmd("com.apple.finder", Launch::Reveal { path })), ["open", "-R", "/repo/a.rs"]);
        let args = ["--line", "3", "/repo/a.rs"].map(String::from).into();
        assert_eq!(
            words(&cmd("com.jetbrains.PhpStorm", Launch::Args { args })),
            ["open", "-n", "-b", "com.jetbrains.PhpStorm", "--args", "--line", "3", "/repo/a.rs"]
        );
    }

    #[test]
    fn an_app_without_its_command_line_gets_the_file_without_the_line() {
        // No such app is installed (or this isn't a Mac): the bundled command line is not found.
        let launch = Launch::Bundled { relative: "Contents/MacOS/cli", arg: "/repo/a.rs:3".into() };
        assert_eq!(
            words(&cmd("com.example.NotInstalled", launch)),
            ["open", "-b", "com.example.NotInstalled", "/repo/a.rs"]
        );
    }
}

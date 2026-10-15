//! Open in…: which apps a terminal's folder can be opened in on this
//! Mac, what their icons look like, and opening a folder in one. Which apps are offered and which
//! one a click picks are core's `open_in`; this asks the system what is installed (Launch Services,
//! by bundle identifier) and draws the icons.
//!
//! Nothing here runs anything from a project: the person asks an app they chose to open a folder,
//! as when they drag it onto the app, so project trust (`ProjectHub::enabled`) doesn't come into it.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use gpui_kit::*;
use wormhole_core::open_in::{self, Installed, Resolved};

use crate::settings::settings;

/// The apps found on this Mac, and the icons drawn for them so far. A GPUI global, set up when the
/// app starts and looked at again whenever a menu that lists the apps opens.
pub(crate) struct OpenIn {
    /// The bundle identifiers of the table's apps that are installed.
    found: Vec<String>,
    /// The icons, by bundle identifier: drawn once, `None` for an app whose icon can't be read.
    icons: RefCell<HashMap<String, Option<Arc<Image>>>>,
}

impl Global for OpenIn {}

/// The icon is drawn this many pixels square: four times the 16 pt it is shown at, so it stays
/// sharp on a Retina display and when the window is scaled.
const ICON_PX: usize = 64;

impl OpenIn {
    /// Look for the apps and keep the result as a global.
    pub(crate) fn install(cx: &mut App) {
        let found = system::installed();
        cx.set_global(Self { found, icons: RefCell::new(HashMap::new()) });
    }

    /// Look again (an app was installed or removed since): the icons already drawn are kept.
    pub(crate) fn refresh(cx: &mut App) {
        let found = system::installed();
        if let Some(this) = cx.try_global::<Self>() {
            let icons = this.icons.take();
            cx.set_global(Self { found, icons: RefCell::new(icons) });
        } else {
            cx.set_global(Self { found, icons: RefCell::new(HashMap::new()) });
        }
    }

    /// The apps the menus list: the table's installed ones, the chosen one if it is a stranger to
    /// the table but installed, and Finder.
    pub(crate) fn apps(cx: &App) -> Vec<Installed> {
        let found = cx.try_global::<Self>().map(|this| this.found.clone()).unwrap_or_default();
        let chosen = settings(cx).open_in.trim().to_string();
        let stranger = (!chosen.is_empty() && open_in::known(&chosen).is_none())
            .then(|| system::name_of(&chosen))
            .flatten()
            .map(|name| Installed { monogram: open_in::monogram(&name), bundle: chosen, name });
        open_in::listing(&found, stranger)
    }

    /// What a click on the title row's button opens.
    pub(crate) fn resolved(cx: &App) -> Resolved {
        open_in::resolve(&settings(cx).open_in, &Self::apps(cx))
    }

    /// The app's own icon, drawn at [`ICON_PX`]; `None` when it can't be read (the button shows
    /// the app's monogram instead).
    pub(crate) fn icon(cx: &App, bundle: &str) -> Option<Arc<Image>> {
        let this = cx.try_global::<Self>()?;
        if let Some(icon) = this.icons.borrow().get(bundle) {
            return icon.clone();
        }
        let icon =
            system::icon_png(bundle, ICON_PX).map(|png| Arc::new(Image::from_bytes(ImageFormat::Png, png)));
        this.icons.borrow_mut().insert(bundle.to_string(), icon.clone());
        icon
    }
}

/// Open `folder` in the app with this bundle identifier (Finder shows it as a window).
pub(crate) fn open(bundle: &str, folder: &Path) {
    let mut cmd = std::process::Command::new("open");
    // A folder is always absolute here, so it can't be read as an option.
    cmd.arg("-b").arg(bundle).arg(folder);
    if let Err(err) = super::spawn::spawn_detached(cmd) {
        tracing::warn!("opening {} in {bundle}: {err}", folder.display());
    }
}

/// Where the app with this bundle identifier is installed.
pub(crate) fn app_path(bundle: &str) -> Option<std::path::PathBuf> {
    system::app_path(bundle)
}

/// The bundle identifier of the app at `path` (an `.app` the person picked), if it is one.
pub(crate) fn bundle_of(path: &Path) -> Option<String> {
    system::bundle_of(path)
}

#[cfg(target_os = "macos")]
mod system {
    use std::path::{Path, PathBuf};

    use objc2::AnyThread as _;
    use objc2::rc::Retained;
    use objc2_app_kit::{
        NSBitmapImageFileType, NSBitmapImageRep, NSCompositingOperation, NSDeviceRGBColorSpace,
        NSGraphicsContext, NSWorkspace,
    };
    use objc2_foundation::{NSBundle, NSDictionary, NSPoint, NSRect, NSSize, NSString};
    use wormhole_core::open_in::EDITORS;

    /// The table's apps that Launch Services knows by their bundle identifier.
    pub(super) fn installed() -> Vec<String> {
        EDITORS
            .iter()
            .filter(|editor| app_path(editor.bundle).is_some())
            .map(|e| e.bundle.to_string())
            .collect()
    }

    pub(super) fn app_path(bundle: &str) -> Option<PathBuf> {
        let url = NSWorkspace::sharedWorkspace()
            .URLForApplicationWithBundleIdentifier(&NSString::from_str(bundle))?;
        url.path().map(|path| PathBuf::from(path.to_string()))
    }

    /// The app's name as Finder shows it: its folder's name without `.app`.
    pub(super) fn name_of(bundle: &str) -> Option<String> {
        let path = app_path(bundle)?;
        Some(path.file_stem()?.to_string_lossy().into_owned())
    }

    pub(super) fn bundle_of(path: &Path) -> Option<String> {
        let bundle = NSBundle::bundleWithPath(&NSString::from_str(&path.to_string_lossy()))?;
        bundle.bundleIdentifier().map(|id| id.to_string())
    }

    /// The icon of the app with this bundle identifier as a PNG, `size` pixels square.
    #[allow(unsafe_code)] // AppKit's bitmap initializer takes raw plane pointers; see the SAFETY note.
    pub(super) fn icon_png(bundle: &str, size: usize) -> Option<Vec<u8>> {
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
        let data = unsafe {
            rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
        }?;
        Some(data.to_vec())
    }
}

/// Off macOS nothing is installed (Finder is always offered, and nothing can be launched).
#[cfg(not(target_os = "macos"))]
mod system {
    use std::path::{Path, PathBuf};

    pub(super) fn installed() -> Vec<String> {
        Vec::new()
    }

    pub(super) fn name_of(_: &str) -> Option<String> {
        None
    }

    pub(super) fn bundle_of(_: &Path) -> Option<String> {
        None
    }

    pub(super) fn icon_png(_: &str, _: usize) -> Option<Vec<u8>> {
        None
    }

    #[allow(dead_code)]
    pub(super) fn app_path(_: &str) -> Option<PathBuf> {
        None
    }
}

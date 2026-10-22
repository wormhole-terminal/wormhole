//! Launch Services' side of `platform::default_app`: the app with the shell role for a content
//! type.

use std::ptr::NonNull;

use objc2_core_foundation::{CFRetained, CFString};
use objc2_foundation::NSBundle;

/// `kLSRolesShell`: the app that runs a file, as Terminal runs a `.command`.
const ROLES_SHELL: u32 = 0x0000_0008;

pub fn own_bundle() -> Option<String> {
    NSBundle::mainBundle().bundleIdentifier().map(|id| id.to_string())
}

/// The bundle identifier of the app that runs files of `kind`.
#[allow(unsafe_code)] // Launch Services' C call; see the SAFETY note.
pub fn runner_for(kind: &str) -> Option<String> {
    #[link(name = "CoreServices", kind = "framework")]
    unsafe extern "C" {
        fn LSCopyDefaultRoleHandlerForContentType(
            content_type: &CFString,
            role: u32,
        ) -> Option<NonNull<CFString>>;
    }
    let kind = CFString::from_str(kind);
    // SAFETY: the content type is a valid CFString for the call; the handler it returns
    // follows the Copy rule (owned by us), so `from_raw` takes that one reference over.
    let handler =
        unsafe { CFRetained::from_raw(LSCopyDefaultRoleHandlerForContentType(&kind, ROLES_SHELL)?) };
    Some(handler.to_string())
}

/// Make the app with `bundle` the one that runs files of `kind`.
#[allow(unsafe_code)] // Launch Services' C call; see the SAFETY note.
pub fn set_runner_for(kind: &str, bundle: &str) -> anyhow::Result<()> {
    #[link(name = "CoreServices", kind = "framework")]
    unsafe extern "C" {
        fn LSSetDefaultRoleHandlerForContentType(
            content_type: &CFString,
            role: u32,
            handler: &CFString,
        ) -> i32;
    }
    let (kind_cf, bundle_cf) = (CFString::from_str(kind), CFString::from_str(bundle));
    // SAFETY: both are valid CFStrings for the call, which only reads them.
    let status = unsafe { LSSetDefaultRoleHandlerForContentType(&kind_cf, ROLES_SHELL, &bundle_cf) };
    anyhow::ensure!(status == 0, "Launch Services refused {kind} (OSStatus {status})");
    Ok(())
}

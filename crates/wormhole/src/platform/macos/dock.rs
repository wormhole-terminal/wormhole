//! AppKit's side of `platform::dock`: the Dock tile's badge, a bounce, whether the app is active,
//! and the activation policy.

use gpui_kit::App;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSRequestUserAttentionType};
use objc2_foundation::NSString;

pub fn set_badge(label: Option<&str>) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let label = label.map(NSString::from_str);
    NSApplication::sharedApplication(mtm).dockTile().setBadgeLabel(label.as_deref());
}

pub fn bounce(_: &mut App) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    NSApplication::sharedApplication(mtm)
        .requestUserAttention(NSRequestUserAttentionType::InformationalRequest);
}

pub fn is_active() -> bool {
    MainThreadMarker::new().is_some_and(|mtm| NSApplication::sharedApplication(mtm).isActive())
}

pub fn hide_icon() {
    let Some(mtm) = MainThreadMarker::new() else { return };
    if !NSApplication::sharedApplication(mtm).setActivationPolicy(NSApplicationActivationPolicy::Accessory) {
        tracing::warn!("background mode: could not drop the Dock icon");
    }
}

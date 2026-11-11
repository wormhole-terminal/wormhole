// About Wormhole: the system's own panel, as Terminal and Finder show it (the GPUI app draws its own window to
// look the same). It is the standard panel, so it brings VoiceOver, the Esc and ⌘W keys and the platform's
// look without a line of ours.

import AppKit
import AuxModel
import DesignSystem

@MainActor
public enum AboutPanel {
    public static func show(_ info: AppInfo = .from(bundle: .main)) {
        let options = Dictionary(uniqueKeysWithValues: info.aboutPanelOptions.map { (NSApplication.AboutPanelOptionKey(rawValue: $0.key), $0.value) })
        let before = Set(NSApp.windows.filter(\.isVisible).map(ObjectIdentifier.init))
        NSApp.orderFrontStandardAboutPanel(options: options)
        // A test instance opens it behind the others and stays in the background.
        if TestInstance.isBackground {
            for window in NSApp.windows where window.isVisible && !before.contains(ObjectIdentifier(window)) { window.orderBack(nil) }
        } else {
            NSApp.activate()
        }
    }
}

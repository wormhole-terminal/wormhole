// About Wormhole: the system's own panel, as Terminal and Finder show it (the GPUI app draws its own window to
// look the same). It is the standard panel, so it brings VoiceOver, the Esc and ⌘W keys and the platform's
// look without a line of ours.

import AppKit
import AuxModel

@MainActor
public enum AboutPanel {
    public static func show(_ info: AppInfo = .from(bundle: .main)) {
        let options = Dictionary(uniqueKeysWithValues: info.aboutPanelOptions.map { (NSApplication.AboutPanelOptionKey(rawValue: $0.key), $0.value) })
        NSApp.orderFrontStandardAboutPanel(options: options)
        NSApp.activate()
    }
}

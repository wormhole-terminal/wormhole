// The Settings window: a titled window with a transparent title bar over the nav, as the GPUI one. It shows one
// `SettingsView` coloured live by `ThemeStore`; a backend change reaches the store through `ThemeSync`. Esc and ⌘W
// close it. ⌘, belongs to the app's menu, which calls `show()`.

import AppKit
import DesignSystem
import SettingsModel
import SwiftUI

@MainActor
public final class SettingsWindowController: NSWindowController {
    public let model: SettingsModel
    private var observation: SettingsObservation?

    public init(model: SettingsModel, themeStore: ThemeStore = .shared) {
        self.model = model
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 980, height: 720),
            styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
            backing: .buffered, defer: false)
        window.title = SettingsText.settings
        window.titlebarAppearsTransparent = true
        window.titleVisibility = .hidden
        window.minSize = NSSize(width: 760, height: 520)
        window.isReleasedWhenClosed = false
        window.setFrameAutosaveName("WormholeSettings")
        super.init(window: window)
        window.contentView = NSHostingView(
            rootView: SettingsView(model: model) { [weak window] in window?.performClose(nil) }.themed(themeStore))
        ThemeSync.apply(model.values, to: themeStore)
        observation = model.backend.observe { [weak model] in
            if let model { ThemeSync.apply(model.values, to: themeStore) }
        }
    }

    @available(*, unavailable) public required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }

    /// Opens the window (or brings it forward) on `section` when given.
    public func show(section: String? = nil) {
        if let section { model.show(section) }
        showWindow(nil)
        window?.makeKeyAndOrderFront(nil)
    }
}

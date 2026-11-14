// The Settings window: a titled window with a transparent title bar over the nav, as the GPUI one. It shows one
// `SettingsView` coloured live by `ThemeStore`; a backend change reaches the store through `ThemeSync`. Esc and ⌘W
// close it. ⌘, belongs to the app's menu, which calls `show()`.

import AppKit
import AuxModel
import DesignSystem
import SettingsModel
import SwiftUI

@MainActor
public final class SettingsWindowController: NSWindowController, NSWindowDelegate {
    public let model: SettingsModel
    private var observation: SettingsObservation?

    public init(model: SettingsModel, updates: UpdateViewModel? = nil, themeStore: ThemeStore = .shared) {
        self.model = model
        let window = NSWindow(
            contentRect: NSRect(origin: .zero, size: SettingsMetric.windowSize),
            styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
            backing: .buffered, defer: false)
        window.title = SettingsText.settings
        window.titlebarAppearsTransparent = true
        window.titleVisibility = .hidden
        window.minSize = NSSize(width: SettingsMetric.windowMin.width, height: SettingsMetric.windowMin.height)
        // The page's column is as wide as it goes: a wider window would only add empty space (`WINDOW_MAX_W`).
        window.maxSize = NSSize(width: SettingsMetric.windowMaxWidth, height: .greatestFiniteMagnitude)
        // The Settings window is never full screen (`forbid_fullscreen`).
        window.collectionBehavior.insert(.fullScreenNone)
        window.isReleasedWhenClosed = false
        window.setFrameAutosaveName("WormholeSettings") // text: an autosave name
        super.init(window: window)
        window.contentView = NSHostingView(
            rootView: SettingsView(model: model) { [weak window] in window?.performClose(nil) }.environment(\.settingsUpdates, updates).themed(themeStore))
        ThemeSync.apply(model.values, to: themeStore)
        observation = model.backend.observe { [weak model] in
            if let model { ThemeSync.apply(model.values, to: themeStore) }
        }
        window.delegate = self
    }

    /// Closed, the window is kept: what its pages watch ends now, and starts again in `show`.
    public func windowWillClose(_ notification: Notification) { model.windowShown = false }

    @available(*, unavailable) public required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }

    /// Opens the window (or brings it forward) on `section` when given.
    public func show(section: String? = nil) {
        if let section { model.show(section) }
        model.windowShown = true
        model.host.windowOpened()
        model.profile.reload()
        TestInstance.bring(window)
    }
}

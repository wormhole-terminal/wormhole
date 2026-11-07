// Renders a SwiftUI view to a PNG through a real (never shown) window, so layout, ScrollView and the appearance
// behave as in the app (ImageRenderer draws none of the scrolling containers). The theme is a bundled one by id,
// with or without Increase Contrast, put in the environment as the window's root does.

import AppKit
import DesignSystem
import SwiftUI

@MainActor
enum Snapshot {
    static let directory = URL(
        fileURLWithPath: ProcessInfo.processInfo.environment["SETTINGS_SNAPSHOTS"]
            ?? "/private/tmp/claude-501/-Users-benjamin-Projects-terminal/e0b38e0a-a3f5-44c8-b6a2-924540bddb46/scratchpad/swift-g-settings")

    @discardableResult
    static func write<V: View>(
        _ view: V, name: String, theme id: String, contrast: Bool = false, size: CGSize = CGSize(width: 1100, height: 760)
    ) throws -> URL {
        guard let theme = ThemeLibrary.bundled.theme(id: id) else { throw CocoaError(.fileNoSuchFile) }
        let tokens = theme.tokens(increaseContrast: contrast)
        let dark = theme.appearance == .dark
        // The scenes read the store for the theme picked, so it follows the theme the page is drawn in.
        ThemeStore.shared.mode = dark ? .dark : .light
        ThemeStore.shared.setTheme(id)
        ThemeStore.shared.increaseContrast = contrast
        let content = view
            .frame(width: size.width, height: size.height)
            .background(tokens.surfaceWindow)
            .environment(\.tokens, tokens)
            .environment(\.colorScheme, dark ? .dark : .light)
        let host = NSHostingView(rootView: content)
        host.frame = CGRect(origin: .zero, size: size)
        let window = NSWindow(contentRect: host.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.appearance = NSAppearance(named: dark ? .darkAqua : .aqua)
        window.contentView = host
        host.layoutSubtreeIfNeeded()
        RunLoop.current.run(until: Date().addingTimeInterval(0.5))
        host.layoutSubtreeIfNeeded()
        guard let rep = host.bitmapImageRepForCachingDisplay(in: host.bounds) else { throw CocoaError(.fileWriteUnknown) }
        host.cacheDisplay(in: host.bounds, to: rep)
        defer { ThemeStore.shared.increaseContrast = false }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let url = directory.appendingPathComponent("\(name)-\(id)\(contrast ? "-hc" : "").png")
        guard let png = rep.representation(using: .png, properties: [:]) else { throw CocoaError(.fileWriteUnknown) }
        try png.write(to: url)
        return url
    }
}

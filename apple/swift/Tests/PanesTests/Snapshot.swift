// Renders a SwiftUI view to a PNG through a real (never shown) window, as the panel's tests do, so layout and
// the appearance behave as in the app. The theme is a bundled one by id, with or without Increase Contrast.

import AppKit
import DesignSystem
import SwiftUI

@MainActor
enum Snapshot {
    static let directory = URL(
        fileURLWithPath: ProcessInfo.processInfo.environment["PANES_SNAPSHOTS"]
            ?? "/private/tmp/claude-501/-Users-benjamin-Projects-terminal/e0b38e0a-a3f5-44c8-b6a2-924540bddb46/scratchpad/g-panes")

    @discardableResult
    static func write<V: View>(
        _ view: V, name: String, theme id: String, contrast: Bool = false, size: CGSize = CGSize(width: 900, height: 600)
    ) throws -> URL {
        guard let theme = ThemeLibrary.bundled.theme(id: id) else { throw CocoaError(.fileNoSuchFile) }
        let tokens = theme.tokens(increaseContrast: contrast)
        let dark = theme.appearance == .dark
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
        RunLoop.current.run(until: Date().addingTimeInterval(0.4))
        host.layoutSubtreeIfNeeded()
        guard let rep = host.bitmapImageRepForCachingDisplay(in: host.bounds) else { throw CocoaError(.fileWriteUnknown) }
        host.cacheDisplay(in: host.bounds, to: rep)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let url = directory.appendingPathComponent("\(name)-\(id)\(contrast ? "-hc" : "").png")
        guard let png = rep.representation(using: .png, properties: [:]) else { throw CocoaError(.fileWriteUnknown) }
        try png.write(to: url)
        return url
    }
}

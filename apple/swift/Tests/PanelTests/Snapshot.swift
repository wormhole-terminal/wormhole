// Renders a SwiftUI view to a PNG through a real (never shown) window, so layout, ScrollView, TextEditor and
// the appearance behave as in the app; ImageRenderer draws none of the scrolling containers.

import AppKit
import SwiftUI

@MainActor
enum Snapshot {
    static let directory = URL(
        fileURLWithPath: ProcessInfo.processInfo.environment["PANEL_SNAPSHOTS"]
            ?? "/private/tmp/claude-501/-Users-benjamin-Projects-terminal/e0b38e0a-a3f5-44c8-b6a2-924540bddb46/scratchpad/panel")

    @discardableResult
    static func write<V: View>(_ view: V, name: String, dark: Bool, size: CGSize = CGSize(width: 340, height: 620)) throws -> URL {
        let appearance = NSAppearance(named: dark ? .darkAqua : .aqua)
        let content = view
            .frame(width: size.width, height: size.height)
            .background(Color(nsColor: .windowBackgroundColor))
            .environment(\.colorScheme, dark ? .dark : .light)
        let host = NSHostingView(rootView: content)
        host.frame = CGRect(origin: .zero, size: size)
        let window = NSWindow(contentRect: host.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.appearance = appearance
        window.contentView = host
        host.layoutSubtreeIfNeeded()
        RunLoop.current.run(until: Date().addingTimeInterval(0.3))
        host.layoutSubtreeIfNeeded()
        guard let rep = host.bitmapImageRepForCachingDisplay(in: host.bounds) else { throw CocoaError(.fileWriteUnknown) }
        host.cacheDisplay(in: host.bounds, to: rep)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let url = directory.appendingPathComponent("\(name)-\(dark ? "dark" : "light").png")
        guard let png = rep.representation(using: .png, properties: [:]) else { throw CocoaError(.fileWriteUnknown) }
        try png.write(to: url)
        return url
    }
}

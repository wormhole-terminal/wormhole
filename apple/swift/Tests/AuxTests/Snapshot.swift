// Renders a SwiftUI view to a PNG through a real (never shown) window, so layout, ScrollView and the appearance
// behave as in the app. The theme is a bundled one by id, with or without Increase Contrast, made the live theme
// (the views read the store through `Palette` and `Row`) and put in the environment as the window's root does.

import AppKit
import DesignSystem
import SwiftUI

@MainActor
enum Snapshot {
    static let directory = URL(
        fileURLWithPath: ProcessInfo.processInfo.environment["AUX_SNAPSHOTS"]
            ?? NSTemporaryDirectory() + "wormhole-swift-shots/swift-g-aux")

    @discardableResult
    static func write<V: View>(
        _ view: V, name: String, theme id: String, contrast: Bool = false, reduceTransparency: Bool = false,
        size: CGSize = CGSize(width: 960, height: 720)
    ) throws -> URL {
        _ = NSApplication.shared
        let store = ThemeStore.shared
        store.show(id)
        store.mode = ThemeLibrary.bundled.theme(id: id)?.appearance == .dark ? .dark : .light
        store.increaseContrast = contrast
        store.reduceTransparency = reduceTransparency
        let dark = store.appearance == .dark
        let host = NSHostingView(rootView: view.frame(width: size.width, height: size.height).themed(store))
        host.frame = CGRect(origin: .zero, size: size)
        let window = NSWindow(contentRect: host.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.appearance = NSAppearance(named: dark ? .darkAqua : .aqua)
        window.contentView = host
        host.layoutSubtreeIfNeeded()
        RunLoop.current.run(until: Date().addingTimeInterval(0.5))
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

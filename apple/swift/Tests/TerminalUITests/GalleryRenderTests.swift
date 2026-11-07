import AppKit
import DesignSystem
import SwiftUI
import XCTest

@testable import TerminalUI

/// Writes the gallery per theme (its own appearance) and once under Increase Contrast to a folder to look at:
/// `TERMINAL_UI_RENDERS` or `out/terminal-ui`. A hosting view in a never-shown window, as `ImageRenderer` skips
/// the text field and the scroll containers.
@MainActor
final class GalleryRenderTests: XCTestCase {
    private var directory: URL {
        URL(fileURLWithPath: ProcessInfo.processInfo.environment["TERMINAL_UI_RENDERS"]
            ?? FileManager.default.currentDirectoryPath + "/out/terminal-ui")
    }

    private func render(theme id: String, contrast: Bool, reduceTransparency: Bool = false, name: String) async throws {
        let store = ThemeStore(follows: false)
        store.setTheme(id)
        store.increaseContrast = contrast
        store.reduceTransparency = reduceTransparency
        // The chip reads the live store (the system's setting, followed by the window), so the shared one is set too.
        ThemeStore.shared.reduceTransparency = reduceTransparency
        defer { ThemeStore.shared.reduceTransparency = false }
        let model = GalleryModel()
        let view = TerminalUIGallery(model: model).themed(store)
        let host = NSHostingView(rootView: AnyView(view))
        host.frame = CGRect(x: 0, y: 0, width: 1240, height: 940)
        let window = NSWindow(contentRect: host.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.appearance = NSAppearance(named: store.appearance == .dark ? .darkAqua : .aqua)
        window.contentView = host
        window.orderBack(nil)
        await model.stage()
        // The chip's spring takes ~0.3 s to settle; the size readout holds 0.7 s.
        for _ in 0..<4 {
            try await Task.sleep(for: .milliseconds(100))
            host.layoutSubtreeIfNeeded()
        }
        let rep = try XCTUnwrap(host.bitmapImageRepForCachingDisplay(in: host.bounds))
        host.cacheDisplay(in: host.bounds, to: rep)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try XCTUnwrap(rep.representation(using: .png, properties: [:])).write(to: directory.appendingPathComponent("\(name).png"))
        window.orderOut(nil)
    }

    func testGalleryInThreeThemesAndContrast() async throws {
        let library = ThemeLibrary.bundled
        for id in [library.defaultDark, library.defaultLight, "tokyo_night", "catppuccin_latte", "gruvbox_dark"] {
            try await render(theme: id, contrast: false, name: id)
        }
        try await render(theme: library.defaultDark, contrast: true, name: "\(library.defaultDark)-contrast")
        try await render(theme: library.defaultDark, contrast: false, reduceTransparency: true, name: "\(library.defaultDark)-opaque")
    }
}

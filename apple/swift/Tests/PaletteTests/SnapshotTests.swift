import AppKit
import SwiftUI
import Testing
import WorkspaceModel

@testable import PaletteUI

/// Renders both overlays over a stand-in window so a person can look at them. A PNG cannot show motion.
@MainActor @Suite struct SnapshotTests {
    private static let session = "/private/tmp/claude-501/-Users-benjamin-Projects-terminal"
    private static let folder = URL(
        fileURLWithPath: session + "/e0b38e0a-a3f5-44c8-b6a2-924540bddb46/scratchpad/palette")

    private func window<V: View>(_ overlay: V, dark: Bool) -> some View {
        ZStack {
            HStack(spacing: 0) {
                Rectangle().fill(Color(nsColor: .underPageBackgroundColor)).frame(width: 220)
                Color(nsColor: .textBackgroundColor)
            }
            overlay
        }
        .frame(width: 900, height: 600)
        .environment(\.colorScheme, dark ? .dark : .light)
    }

    /// Through a real (off-screen) window and `cacheDisplay`: `ImageRenderer` draws AppKit-backed controls
    /// (the text field, the scroll view) as yellow placeholders.
    private func write<V: View>(_ view: V, named name: String, dark: Bool) throws {
        let host = NSHostingView(rootView: view)
        let frame = NSRect(x: 0, y: 0, width: 900, height: 600)
        let window = NSWindow(contentRect: frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.appearance = NSAppearance(named: dark ? .darkAqua : .aqua)
        window.contentView = host
        window.layoutIfNeeded()
        RunLoop.current.run(until: Date().addingTimeInterval(0.3))
        let rep = try #require(host.bitmapImageRepForCachingDisplay(in: frame))
        host.cacheDisplay(in: frame, to: rep)
        let png = try #require(rep.representation(using: .png, properties: [:]))
        try FileManager.default.createDirectory(at: Self.folder, withIntermediateDirectories: true)
        try png.write(to: Self.folder.appendingPathComponent(name))
    }

    private var state: WorkspaceState {
        WorkspaceState(
            terminals: [
                TerminalSummary(id: 1, title: "api server", subtitle: "~/api", branch: "main", activity: .working),
                TerminalSummary(id: 2, title: "web", subtitle: "~/web", branch: "redesign"),
                TerminalSummary(id: 3, title: "notes", subtitle: "~/notes"),
            ], selected: 2)
    }

    @Test func paletteImages() throws {
        for dark in [false, true] {
            let tone = dark ? "dark" : "light"
            for (name, query) in [("empty", ""), ("typed", "sid"), ("scoped", "@we"), ("none", "zzq")] {
                let view = PaletteView(state: state, send: { _ in }, dismiss: {}, startsPresented: true, query: query)
                try write(window(view, dark: dark), named: "palette-\(name)-\(tone).png", dark: dark)
            }
        }
    }

    @Test func sheetImages() throws {
        for dark in [false, true] {
            let sheet = ShortcutSheetView(dismiss: {}, startsPresented: true)
            try write(window(sheet, dark: dark), named: "sheet-\(dark ? "dark" : "light").png", dark: dark)
        }
        let filtered = ShortcutSheetView(dismiss: {}, startsPresented: true, filter: "⌘")
        try write(window(filtered, dark: false), named: "sheet-filtered-light.png", dark: false)
    }
}

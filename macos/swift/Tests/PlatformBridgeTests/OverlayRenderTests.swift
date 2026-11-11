import AppKit
import DesignSystem
import PaletteModel
import SwiftUI
import Testing
import WorkspaceModel

@testable import PaletteUI
@testable import PlatformBridge

/// The palette and the shortcut sheet drawn from core's own lists over the real FFI (not the sample's), per theme, so a
/// person can look at them next to the GPUI app's screenshots (`gpui-ref/dark-sheet.png`, `dark-palette.png`).
@MainActor @Suite(.serialized) struct OverlayRenderTests {
    private static let folder = URL(
        fileURLWithPath: NSTemporaryDirectory() + "wormhole-swift-shots/s-overlays/shots")
    private static let size = CGSize(width: 1320, height: 820)

    private func write<V: View>(_ overlay: V, named name: String, theme id: String) throws {
        let theme = try #require(ThemeLibrary.bundled.theme(id: id))
        let tokens = theme.tokens(increaseContrast: false)
        let scene = ZStack {
            HStack(spacing: 0) {
                tokens.sidebarBackground.frame(width: 260)
                tokens.surfaceWindow
                tokens.surfacePanel.frame(width: 260)
            }
            overlay
        }
        .frame(width: Self.size.width, height: Self.size.height)
        .environment(\.tokens, tokens)
        .environment(\.colorScheme, theme.appearance.colorScheme)
        let host = NSHostingView(rootView: scene)
        let frame = NSRect(origin: .zero, size: Self.size)
        let window = NSWindow(contentRect: frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.appearance = NSAppearance(named: theme.appearance == .dark ? .darkAqua : .aqua)
        window.contentView = host
        window.layoutIfNeeded()
        RunLoop.current.run(until: Date().addingTimeInterval(0.3))
        let rep = try #require(host.bitmapImageRepForCachingDisplay(in: frame))
        host.cacheDisplay(in: frame, to: rep)
        try FileManager.default.createDirectory(at: Self.folder, withIntermediateDirectories: true)
        try #require(rep.representation(using: .png, properties: [:])).write(to: Self.folder.appendingPathComponent("\(id)-\(name).png"))
    }

    private var state: WorkspaceState {
        WorkspaceState(
            terminals: [
                TerminalSummary(id: 1, title: "zsh", folder: "~/scratchpad/ref-repo", isPinned: true),
                TerminalSummary(id: 3, title: "sidebar polish", folder: "~/scratchpad/ref-repo", branch: "main", activity: .working),
                TerminalSummary(id: 4, title: "diff perf", folder: "~/scratchpad/ref-repo", activity: .needsInput),
            ], selected: 1)
    }

    @Test func overlaysFromTheCoresLists() throws {
        ListedKeymap.install(CoreKeymapLists())
        let sources = PaletteSources(actions: { ListedKeymap.actions(ActionFacts()) })
        for theme in ["wormhole.dark", "wormhole.light"] {
            try write(ShortcutSheetView(hidden: [], hotkey: "⌃⌥Space", dismiss: {}, startsPresented: true), named: "sheet", theme: theme)
            try write(ShortcutSheetView(hidden: [], hotkey: nil, dismiss: {}, startsPresented: true, filter: "⌘"), named: "sheet-filtered", theme: theme)
            for (name, query) in [("palette", ""), ("palette-actions", ">"), ("palette-query", "split")] {
                try write(
                    PaletteView(state: state, send: { _ in }, sources: sources, dismiss: {}, startsPresented: true, query: query),
                    named: name, theme: theme)
            }
        }
    }
}

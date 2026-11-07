import AppKit
import DesignSystem
import SwiftUI
import Testing
import WorkspaceModel

@testable import PaletteUI

/// Renders both overlays over a stand-in window, per theme and contrast, so a person can look at them next to
/// the GPUI app's own screenshots. A PNG cannot show motion.
@MainActor @Suite(.serialized) struct SnapshotTests {
    private static let folder = URL(
        fileURLWithPath: "/private/tmp/claude-501/-Users-benjamin-Projects-terminal/e0b38e0a-a3f5-44c8-b6a2-924540bddb46/scratchpad/swift-palette")
    private static let size = CGSize(width: 1320, height: 820)
    /// Light and dark, the default theme and others, so a colour that was written instead of read shows.
    private static let themes = ["wormhole.dark", "wormhole.light", "nord", "github_light", "dracula", "soltari.light"]

    private func tokens(_ id: String, contrast: Bool) throws -> (ThemeTokens, ThemeAppearance) {
        let theme = try #require(ThemeLibrary.bundled.theme(id: id))
        return (theme.tokens(increaseContrast: contrast), theme.appearance)
    }

    private func write<V: View>(
        _ overlay: V, named name: String, theme: String, contrast: Bool = false, reduceTransparency: Bool = false
    ) throws {
        let (tokens, appearance) = try tokens(theme, contrast: contrast)
        ThemeStore.shared.increaseContrast = contrast
        defer { ThemeStore.shared.increaseContrast = false }
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
        .environment(\.colorScheme, appearance.colorScheme)
        let host = NSHostingView(rootView: scene)
        let frame = NSRect(origin: .zero, size: Self.size)
        let window = NSWindow(contentRect: frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.appearance = NSAppearance(named: appearance == .dark ? .darkAqua : .aqua)
        window.contentView = host
        window.layoutIfNeeded()
        RunLoop.current.run(until: Date().addingTimeInterval(0.3))
        let rep = try #require(host.bitmapImageRepForCachingDisplay(in: frame))
        host.cacheDisplay(in: frame, to: rep)
        let png = try #require(rep.representation(using: .png, properties: [:]))
        try FileManager.default.createDirectory(at: Self.folder, withIntermediateDirectories: true)
        try png.write(to: Self.folder.appendingPathComponent("\(theme)\(contrast ? "-contrast" : "")-\(name).png"))
    }

    /// The reference window's terminals (`gpui-ref/SPEC.md`).
    private var state: WorkspaceState {
        WorkspaceState(
            terminals: [
                TerminalSummary(id: 1, title: "zsh", subtitle: "~/scratchpad/ref-repo", isPinned: true),
                TerminalSummary(id: 2, title: "build docs", subtitle: "~"),
                TerminalSummary(id: 3, title: "sidebar polish", subtitle: "~/scratchpad/ref-repo", branch: "main", activity: .working),
                TerminalSummary(id: 4, title: "diff perf", subtitle: "~/scratchpad/ref-repo", activity: .needsInput),
                TerminalSummary(id: 5, title: "rate limits", subtitle: "~/scratchpad/ref-repo", activity: .finished),
                TerminalSummary(id: 6, title: "scratch", subtitle: "~"),
            ], selected: 1)
    }

    private func palette(_ query: String, chosen: String? = nil, hovered: String? = nil) -> PaletteView {
        PaletteView(state: state, send: { _ in }, dismiss: {}, startsPresented: true, query: query, chosen: chosen, hovered: hovered)
    }

    @Test func paletteImages() throws {
        for theme in Self.themes {
            for (name, query) in [("palette", ""), ("palette-query", "new"), ("palette-terminals", "@"), ("palette-none", "zzq")] {
                try write(palette(query), named: name, theme: theme)
            }
        }
        try write(palette("", hovered: "panel-changes"), named: "palette-hover", theme: "wormhole.dark")
        for theme in ["wormhole.dark", "wormhole.light"] {
            try write(palette("new"), named: "palette-query", theme: theme, contrast: true)
        }
    }

    @Test func sheetImages() throws {
        for theme in Self.themes {
            try write(ShortcutSheetView(dismiss: {}, startsPresented: true), named: "sheet", theme: theme)
        }
        for theme in ["wormhole.dark", "wormhole.light"] {
            try write(ShortcutSheetView(dismiss: {}, startsPresented: true), named: "sheet", theme: theme, contrast: true)
            try write(ShortcutSheetView(dismiss: {}, startsPresented: true, filter: "⌘"), named: "sheet-filtered", theme: theme)
        }
    }
}

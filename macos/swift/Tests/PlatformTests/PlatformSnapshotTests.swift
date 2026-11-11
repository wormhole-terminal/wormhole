import AppKit
import DesignSystem
import PlatformModel
import SwiftUI
import Testing

@testable import PlatformUI

/// Renders the quick panel, the folder picker, the shortcut recorder and the status icon over a stand-in window, per
/// theme and contrast, so a person can look at them beside the GPUI app's screenshots. A PNG cannot show motion.
@MainActor @Suite(.serialized) struct PlatformSnapshotTests {
    private static let folder = URL(
        fileURLWithPath: NSTemporaryDirectory() + "wormhole-swift-shots/g-platform")
    private static let themes = ["wormhole.dark", "wormhole.light", "nord", "github_light", "dracula"]

    private func tokens(_ id: String, contrast: Bool) throws -> (ThemeTokens, ThemeAppearance) {
        let theme = try #require(ThemeLibrary.bundled.theme(id: id))
        return (theme.tokens(increaseContrast: contrast), theme.appearance)
    }

    private func write<V: View>(
        _ view: V, named name: String, theme: String, contrast: Bool = false, size: CGSize, reduceTransparency: Bool = false
    ) throws {
        let (tokens, appearance) = try tokens(theme, contrast: contrast)
        ThemeStore.shared.increaseContrast = contrast
        ThemeStore.shared.reduceTransparency = reduceTransparency
        defer { ThemeStore.shared.increaseContrast = false; ThemeStore.shared.reduceTransparency = false }
        let scene = ZStack(alignment: .top) {
            HStack(spacing: 0) {
                tokens.sidebarBackground.frame(width: 160)
                tokens.surfaceWindow
            }
            view.padding(.top, 40).shadow(color: tokens.shadow, radius: 24, y: 12)
        }
        .frame(width: size.width, height: size.height)
        .environment(\.tokens, tokens)
        .environment(\.colorScheme, appearance.colorScheme)
        try render(scene, appearance: appearance, size: size, to: "\(theme)\(contrast ? "-contrast" : "")-\(name)")
    }

    private func render<V: View>(_ scene: V, appearance: ThemeAppearance, size: CGSize, to name: String) throws {
        let host = NSHostingView(rootView: scene)
        let frame = NSRect(origin: .zero, size: size)
        let window = NSWindow(contentRect: frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.appearance = NSAppearance(named: appearance == .dark ? .darkAqua : .aqua)
        window.contentView = host
        window.layoutIfNeeded()
        RunLoop.current.run(until: Date().addingTimeInterval(0.3))
        let rep = try #require(host.bitmapImageRepForCachingDisplay(in: frame))
        host.cacheDisplay(in: frame, to: rep)
        let png = try #require(rep.representation(using: .png, properties: [:]))
        try FileManager.default.createDirectory(at: Self.folder, withIntermediateDirectories: true)
        try png.write(to: Self.folder.appendingPathComponent("\(name).png"))
    }

    // MARK: quick panel

    private func quickModel(query: String = "", answering: Bool = false) -> QuickPanelModel {
        let model = QuickPanelModel()
        let now = Date()
        model.now = now
        var diff = AmbientTerminal(session: 4, name: "diff perf", state: .needsYou, since: now - 125, source: .agent, who: "Claude Code", message: "Approve cargo test -p wormhole-core?")
        diff.folder = "ref-repo"
        var tests = AmbientTerminal(session: 3, name: "sidebar polish", state: .working, since: now - 252, source: .agent, who: "Claude Code", message: "Editing sidebar/row.rs")
        tests.branch = "main"
        var build = AmbientTerminal(session: 5, name: "rate limits", state: .done, since: now - 720, source: .agent, who: "Claude Code", changed: 7)
        build.folder = "ref-repo"
        var failed = AmbientTerminal(session: 7, name: "cargo build", state: .failed, since: now - 90, command: "cargo build", took: 38, exit: 101)
        failed.folder = "terminal"
        model.terminals = [
            diff, AmbientTerminal(session: 8, name: "prod", state: .needsYou, since: now - 300, command: "ssh prod"), tests, build, failed,
            AmbientTerminal(session: 1, name: "zsh", state: .idle, folder: "ref-repo"),
        ]
        model.items = [.newTerminal(cwd: "/Users/wanderer/Projects/terminal"), .openFolder, .action(.init(project: "api-server", root: "/src/api", id: "test", title: "test"))]
        model.hotkey = "⇧⌘Space"
        model.query = query
        if answering { model.select(model.rows[1]); model.commandReturn() }
        return model
    }

    @Test func quickPanelImages() throws {
        for theme in Self.themes {
            let model = quickModel()
            try write(QuickPanelView(model: model), named: "quick-panel", theme: theme, size: CGSize(width: 760, height: 620))
        }
        for theme in ["wormhole.dark", "wormhole.light"] {
            try write(QuickPanelView(model: quickModel(query: "diff")), named: "quick-panel-query", theme: theme, size: CGSize(width: 760, height: 360))
            try write(QuickPanelView(model: quickModel(query: "zzq")), named: "quick-panel-none", theme: theme, size: CGSize(width: 760, height: 260))
            try write(QuickPanelView(model: quickModel(answering: true)), named: "quick-panel-reply", theme: theme, size: CGSize(width: 760, height: 620))
            try write(QuickPanelView(model: quickModel()), named: "quick-panel", theme: theme, contrast: true, size: CGSize(width: 760, height: 620))
        }
        let empty = QuickPanelModel()
        empty.items = [.newTerminal(cwd: nil), .openFolder]
        try write(QuickPanelView(model: empty), named: "quick-panel-empty", theme: "wormhole.dark", size: CGSize(width: 760, height: 320))
    }

    // MARK: folder picker

    private func pickerDriver(_ text: String) async throws -> FolderPickerDriver {
        let driver = FolderPickerDriver(
            context: PickerContext(cwd: "/Users/wanderer/Projects/terminal", home: "/Users/wanderer"),
            providers: [MockFolders.sample], decorators: [MockFolders.sample])
        driver.open()
        try await Task.sleep(for: .milliseconds(100))
        if !text.isEmpty {
            driver.typed(text)
            try await Task.sleep(for: .milliseconds(100))
        }
        return driver
    }

    @Test func folderPickerImages() async throws {
        let idle = try await pickerDriver(""), typed = try await pickerDriver("pro"), none = try await pickerDriver("zzq")
        for theme in Self.themes {
            try write(FolderPickerView(driver: idle), named: "folder-picker", theme: theme, size: CGSize(width: 640, height: 480))
        }
        for theme in ["wormhole.dark", "wormhole.light"] {
            try write(FolderPickerView(driver: typed), named: "folder-picker-typed", theme: theme, size: CGSize(width: 640, height: 300))
            try write(FolderPickerView(driver: none), named: "folder-picker-none", theme: theme, size: CGSize(width: 640, height: 220))
            try write(FolderPickerView(driver: idle), named: "folder-picker-opaque", theme: theme, size: CGSize(width: 640, height: 480), reduceTransparency: true)
            try write(FolderPickerView(driver: typed), named: "folder-picker-typed", theme: theme, contrast: true, size: CGSize(width: 640, height: 300))
        }
    }

    // MARK: recorder and icon

    private struct RecorderSheet: View {
        @State var none: KeySpec?
        @State var set: KeySpec? = KeySpec([.shift, .command], "space")
        @Environment(\.tokens) private var tokens

        var body: some View {
            VStack(alignment: .leading, spacing: Space._3) {
                Text("Global hotkey").font(.system(size: TextRole.label.size, weight: .semibold)).foregroundStyle(tokens.textPrimary)
                ShortcutRecorder($set, own: nil)
                ShortcutRecorder($none)
                Text("⌘T is New Terminal").font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.statusDanger)
            }
            .padding(Space._4)
            .frame(width: 320, alignment: .leading)
            .background(tokens.surfacePanel)
        }
    }

    @Test func recorderImages() throws {
        for theme in ["wormhole.dark", "wormhole.light", "nord"] {
            try write(RecorderSheet(), named: "recorder", theme: theme, size: CGSize(width: 360, height: 180))
        }
    }

    @Test func statusIconSheet() throws {
        let colors = StatusIconImage.Colors(attention: .systemOrange, done: .systemGreen, danger: .systemRed)
        let states: [(String, Bool, Int?, StatusMark)] = [
            ("idle", false, nil, .none), ("working", true, nil, .none), ("w0", true, 0, .none), ("w1", true, 1, .none), ("w2", true, 2, .none),
            ("w3", true, 3, .none), ("needs-you", false, nil, .needsYou), ("failed", false, nil, .failed), ("finished", false, nil, .finished),
        ]
        for dark in [true, false] {
            let row = HStack(spacing: 18) {
                ForEach(states, id: \.0) { _, filled, frame, mark in
                    Image(nsImage: StatusIconImage.image(filled: filled, frame: frame, mark: mark, colors: colors, description: ""))
                        .renderingMode(mark == .none ? .template : .original)
                        .resizable().frame(width: 84, height: 64)
                        .foregroundStyle(dark ? Color.white : Color.black)
                }
            }
            .padding(20)
            .background(dark ? Color(white: 0.16) : Color(white: 0.92))
            try render(row, appearance: dark ? .dark : .light, size: CGSize(width: 20 + 9 * 102, height: 104), to: "status-icon-\(dark ? "dark" : "light")")
        }
    }
}

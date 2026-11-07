import Testing
import WorkspaceModel

@testable import PaletteModel

@Suite struct CatalogTests {
    private func command(_ id: String, _ state: WorkspaceState = sample()) -> Command? {
        CommandCatalog.commands(for: state).first { $0.id == id }
    }

    @Test func oneResultPerTerminalWithNumberKeys() {
        let terminals = CommandCatalog.commands(for: sample()).filter { $0.group == .terminals }
        #expect(terminals.map(\.title) == ["api server", "web", "notes"])
        #expect(terminals.map(\.action) == [.select(1), .select(2), .select(3)])
        #expect(terminals.map { $0.shortcut?.glyphs } == ["⌘1", "⌘2", "⌘3"])
        #expect(terminals[0].detail == "~/api")
        #expect(terminals[0].scopedDetail == "Window 1 · ~/api")
        #expect(terminals.allSatisfy { $0.isStrong })
    }

    @Test func onlyNineTerminalsGetANumber() {
        let many = WorkspaceState(terminals: (1...11).map { TerminalSummary(id: $0, title: "t\($0)") })
        let terminals = CommandCatalog.commands(for: many).filter { $0.group == .terminals }
        #expect(terminals[8].shortcut?.glyphs == "⌘9")
        #expect(terminals[9].shortcut == nil)
    }

    @Test func foldersCountTheirTerminals() {
        let state = WorkspaceState(terminals: [
            TerminalSummary(id: 1, title: "a", subtitle: "~/api"), TerminalSummary(id: 2, title: "b", subtitle: "~/api"),
            TerminalSummary(id: 3, title: "c", subtitle: "zsh"),
        ])
        let folders = CommandCatalog.commands(for: state).filter { $0.group == .folders }
        #expect(folders.map(\.title) == ["api"])
        #expect(folders[0].detail == "~ · 2 terminals")
        #expect(folders[0].action == .select(1))
    }

    @Test func togglesNameTheirEffectInSentenceCase() {
        var state = sample()
        #expect(command("toggle-sidebar", state)?.title == "Hide sidebar")
        state.sidebarVisible = false
        #expect(command("toggle-sidebar", state)?.title == "Show sidebar")
        state.panelVisible = false
        #expect(command("toggle-panel", state)?.title == "Show right panel")
    }

    @Test func selectedTerminalCommandsNeedASelection() {
        #expect(command("close-terminal")?.action == .close(2))
        #expect(command("pin-terminal")?.action == .togglePin(2))
        #expect(command("next-terminal")?.action == .select(3))
        #expect(command("previous-terminal")?.action == .select(1))
        var none = sample()
        none.selected = nil
        #expect(command("close-terminal", none) == nil)
    }

    @Test func theCommandPaletteIsNotAnActionOfItself() {
        #expect(command("command-palette") == nil)
        #expect(command("keyboard-shortcuts")?.action == .showOverlay(.shortcuts))
    }

    @Test func everyCommandRuns() {
        #expect(CommandCatalog.commands(for: sample()).filter { $0.group != .folders }.allSatisfy { $0.action != nil || $0.appAction != nil })
    }

    @Test func glyphsFollowAppleOrderAndParseBack() {
        #expect(Shortcut([.command, .shift, .option, .control], "T").glyphs == "⌃⌥⇧⌘T")
        #expect(Shortcut(glyphs: "⇧⌥⌘I") == Shortcut([.shift, .option, .command], "I"))
        #expect(Shortcut(glyphs: "⌘⌘T") == nil)
        #expect(Shortcut(glyphs: "⌘") == nil)
        #expect(Shortcut(glyphs: "F6")?.glyphs == "F6")
    }
}

@Suite struct SheetTests {
    @Test func filtersByLabelAndByKeyAsTheRustSheetDoes() {
        #expect(ShortcutSheet.filter("").sections.count == 3)
        let byKey = ShortcutSheet.filter("⌘W").sections.flatMap { $0.rows.map(\.row.label) }
        #expect(byKey == ["Close terminal"])
        let byLabel = ShortcutSheet.filter("sidebar")
        #expect(byLabel.sections.map(\.title) == ["View & panels"])
        #expect(byLabel.sections[0].rows[0].hit.label == 12..<19)
        #expect(ShortcutSheet.filter("zzzz").shown == 0)
    }

    @Test func aKeyInsideARunFindsTheRun() {
        #expect(ShortcutSheet.filter("⌘5").sections.flatMap { $0.rows.map(\.row.label) } == ["Terminals 1–9"])
        #expect(ShortcutSheet.sections[0].rows[3].keysText == "⌘1 … ⌘9")
    }

    @Test func countsRowsShownAndAll() {
        let all = ShortcutSheet.filter("")
        #expect(all.shown == all.total)
        #expect(ShortcutSheet.filter("new").shown < all.total)
    }

    @Test func columnsMinimiseTheTallest() {
        #expect(SheetColumns.split([5, 5, 5], into: 3) == [0..<1, 1..<2, 2..<3])
        #expect(SheetColumns.split([9, 2, 2, 9], into: 2) == [0..<2, 2..<4])
        #expect(SheetColumns.split([3], into: 3) == [0..<1])
        #expect(SheetColumns.split([], into: 3).isEmpty)
    }
}

func sample() -> WorkspaceState {
    WorkspaceState(
        terminals: [
            TerminalSummary(id: 1, title: "api server", subtitle: "~/api", branch: "main", activity: .working),
            TerminalSummary(id: 2, title: "web", subtitle: "~/web", branch: "redesign"),
            TerminalSummary(id: 3, title: "notes"),
        ], selected: 2)
}

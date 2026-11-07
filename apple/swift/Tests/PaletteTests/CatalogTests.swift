import Testing
import WorkspaceModel

@testable import PaletteModel

@Suite struct CatalogTests {
    private func command(_ id: String, _ state: WorkspaceState = sample()) -> Command? {
        CommandCatalog.commands(for: state).first { $0.id == id }
    }

    @Test func oneGoToEntryPerTerminalWithNumberKeys() {
        let go = CommandCatalog.commands(for: sample()).filter { $0.group == .terminals }
        #expect(go.map(\.title) == ["Go to api server", "Go to web", "Go to notes"])
        #expect(go.map(\.action) == [.select(1), .select(2), .select(3)])
        #expect(go.map { $0.shortcut?.glyphs } == ["⌘1", "⌘2", "⌘3"])
        #expect(go[0].subtitle == "~/api · main")
    }

    @Test func onlyNineTerminalsGetANumber() {
        let many = WorkspaceState(terminals: (1...11).map { TerminalSummary(id: $0, title: "t\($0)") })
        let go = CommandCatalog.commands(for: many).filter { $0.group == .terminals }
        #expect(go[8].shortcut?.glyphs == "⌘9")
        #expect(go[9].shortcut == nil)
    }

    @Test func togglesNameTheirEffect() {
        var state = sample()
        #expect(command("toggle-sidebar", state)?.title == "Hide Sidebar")
        state.sidebarVisible = false
        #expect(command("toggle-sidebar", state)?.title == "Show Sidebar")
        state.panelVisible = false
        #expect(command("toggle-panel", state)?.title == "Show Right Panel")
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

    @Test func everyCatalogCommandRuns() {
        #expect(CommandCatalog.commands(for: sample()).allSatisfy { $0.action != nil })
        #expect(CommandCatalog.commands(for: WorkspaceState()).map(\.id).contains("new-terminal"))
    }

    @Test func panelTabsHaveTheRustKeys() {
        #expect(command("panel-changes")?.shortcut?.glyphs == "⌃⌘1")
        #expect(command("panel-notes")?.shortcut?.glyphs == "⌃⌘3")
    }

    @Test func glyphsFollowAppleOrder() {
        #expect(Shortcut([.command, .shift, .option, .control], "T").glyphs == "⌃⌥⇧⌘T")
        #expect(Shortcuts.commandPalette.parts == ["⇧", "⌘", "P"])
    }

    @Test func sheetFiltersByLabelAndByKeys() {
        #expect(ShortcutSheet.sections(matching: "").count == 3)
        let byKeys = ShortcutSheet.sections(matching: "⌘W").flatMap(\.rows).map(\.label)
        #expect(byKeys.contains("Close terminal"))
        let byLabel = ShortcutSheet.sections(matching: "sidebar")
        #expect(byLabel.map(\.title) == ["View & panels"])
        #expect(ShortcutSheet.sections(matching: "zzzz").isEmpty)
        #expect(ShortcutSheet.sections[0].rows[3].keysText == "⌘1 … ⌘9")
    }
}

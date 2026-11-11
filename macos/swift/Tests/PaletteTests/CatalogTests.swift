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
        #expect(terminals.map(\.target) == [.terminal(1), .terminal(2), .terminal(3)])
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
            TerminalSummary(id: 1, title: "a", folder: "~/api"), TerminalSummary(id: 2, title: "b", folder: "~/api"),
            TerminalSummary(id: 3, title: "c", folder: "zsh"),
        ])
        let folders = CommandCatalog.commands(for: state).filter { $0.group == .folders }
        #expect(folders.map(\.title) == ["api"])
        #expect(folders[0].detail == "~ · 2 terminals")
        #expect(folders[0].target == .folder("~/api"))
    }

    @Test func actionsAreTheCoresListWithItsWordsKeysAndGroups() {
        SampleLists.install()
        let list = ListedKeymap.actions(ActionFacts())
        let actions = CommandCatalog.commands(for: sample(), actions: list).filter { $0.group == .actions }
        #expect(actions.map(\.title).prefix(3) == ["New terminal", "Close terminal", "Open folder…"])
        let new = actions[0]
        #expect((new.id, new.appAction, new.shortcut?.glyphs, new.section) == ("action-newTerminal", "newTerminal", "⌘T", .terminal))
        #expect(new.detail == "in ~/web")
        #expect(actions.map(\.section).contains(.app))
        #expect(actions.allSatisfy { $0.appAction != nil })
    }

    @Test func settingsAndCommitsAreResultsOfTheirOwn() {
        let setting = PaletteSetting(
            id: "opacity", key: "opacity", label: "Opacity", section: "Appearance", icon: "paintpalette", value: "80 %",
            match: Match(score: 9, ranges: [0..<3]))
        let commit = PaletteCommit(sha: "abc1234ffff", short: "abc1234", subject: "fix the tab bar", author: "Ben")
        let found = CommandCatalog.commands(for: sample(), settings: [setting], commits: [commit])
        let one = found.first { $0.group == .settings }
        #expect((one?.detail, one?.trailing, one?.target) == ("Settings › Appearance", "80 %", .setting("opacity")))
        let two = found.first { $0.group == .commits }
        #expect((two?.title, two?.trailing, two?.trailingIsCode, two?.target) == ("fix the tab bar", "abc1234", true, .commit("abc1234ffff")))
        // Found by the settings index, a setting keeps its match and the index's order.
        let rows = PaletteRows.rows(scope: .settings, query: "op", commands: found)
        #expect(rows.compactMap(\.result).first?.ranges == [0..<3])
        #expect(PaletteRows.rows(scope: .commits, query: "tab", commands: found).compactMap(\.result?.id) == ["commit-abc1234ffff"])
    }

    @Test func profilesOfferASwitchAWindowAndAMoveOnlyWithAnotherToGoTo() {
        #expect(CommandCatalog.commands(for: sample(), profiles: [PaletteProfile(name: "Default", isShown: true, isHere: true)]).isEmpty == false)
        let one = [PaletteProfile(name: "Default", isShown: true, isHere: true)]
        #expect(CommandCatalog.commands(for: sample(), profiles: one).filter { $0.section == .profiles }.isEmpty)
        let two = one + [PaletteProfile(name: "Work", isShown: false, isHere: false)]
        let rows = CommandCatalog.commands(for: sample(), profiles: two).filter { $0.section == .profiles }
        #expect(rows.map(\.title) == ["Switch to Work", "Open window in Default", "Open window in Work", "Move this window to Work"])
        #expect(rows.first?.target == .profile(.switchTo, 1))
    }

    @Test func glyphsFollowAppleOrderAndParseBack() {
        #expect(Shortcut([.command, .shift, .option, .control], "T").glyphs == "⌃⌥⇧⌘T")
        #expect(Shortcut(glyphs: "⇧⌥⌘I") == Shortcut([.shift, .option, .command], "I"))
        #expect(Shortcut(glyphs: "⌘⌘T") == nil)
        #expect(Shortcut(glyphs: "⌘") == nil)
        #expect(Shortcut(glyphs: "F6")?.glyphs == "F6")
    }
}

func sample() -> WorkspaceState {
    WorkspaceState(
        terminals: [
            TerminalSummary(id: 1, title: "api server", folder: "~/api", branch: "main", activity: .working),
            TerminalSummary(id: 2, title: "web", folder: "~/web", branch: "redesign"),
            TerminalSummary(id: 3, title: "notes"),
        ], selected: 2)
}

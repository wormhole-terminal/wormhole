import Testing
import WorkspaceModel

@testable import PaletteModel

@Suite struct SearchTests {
    private func score(_ word: String, _ text: String) -> Int? { PaletteSearch.fuzzy(word, in: text)?.score }

    @Test func prefixBeatsWordStartBeatsScattered() throws {
        let prefix = try #require(score("sid", "Sidebar polish"))
        let wordStart = try #require(score("sid", "Hide Sidebar"))
        let inside = try #require(score("sid", "Inside out"))
        #expect(prefix > wordStart)
        #expect(wordStart > inside)
    }

    @Test func lettersInOrderAcrossWordStarts() {
        #expect(score("sp", "sidebar polish") != nil)
        #expect(score("shm", "theme shape metrics") != nil)
        #expect(score("ts", "Terminals") == nil)
    }

    @Test func rangesMarkTheMatchedCharacters() throws {
        let match = try #require(PaletteSearch.fuzzy("hsid", in: "Hide Sidebar"))
        #expect(match.ranges == [0..<1, 5..<8])
    }

    @Test func everyWordMustBeFoundAndOtherTextCountsLess() throws {
        #expect(PaletteSearch.score("go api", title: "Go to build", other: ["~/api · main"]) != nil)
        #expect(PaletteSearch.score("go zzz", title: "Go to build", other: ["~/api"]) == nil)
        let inTitle = try #require(PaletteSearch.score("api", title: "api server"))
        let inOther = try #require(PaletteSearch.score("api", title: "Go to build", other: ["api"]))
        #expect(inTitle.score > inOther.score)
    }

    @Test func prefixesPickTheScope() {
        #expect(PaletteScope.parse("@diff") == (.terminals, "diff"))
        #expect(PaletteScope.parse("> new tab") == (.actions, "new tab"))
        #expect(PaletteScope.parse("~") == (.folders, ""))
        #expect(PaletteScope.parse("#fix") == (.commits, "fix"))
        #expect(PaletteScope.parse(",theme") == (.settings, "theme"))
        #expect(PaletteScope.parse("/style.rs") == (.all, "/style.rs"))
        #expect(PaletteScope.parse("a@b") == (.all, "a@b"))
    }

    @Test func tabWalksTheScopeRowAndWraps() {
        #expect(PaletteScope.all.step(forward: true) == .terminals)
        #expect(PaletteScope.actions.step(forward: true) == .all)
        #expect(PaletteScope.all.step(forward: false) == .actions)
        #expect(PaletteScope.row.map(\.title) == ["All", "Terminals", "Folders", "Commits", "Settings", "Actions"])
    }
}

@Suite struct RowsTests {
    private let commands = CommandCatalog.commands(for: sample())

    private func rows(_ scope: PaletteScope = .all, _ query: String = "", recent: [String] = []) -> [PaletteRow] {
        PaletteRows.rows(scope: scope, query: query, commands: commands, recent: recent)
    }

    @Test func emptyQuerySuggestsTheActionsThatFit() {
        let rows = rows()
        #expect(rows.first == .label(title: "Actions", count: nil))
        #expect(rows.compactMap(\.result?.id) == CommandCatalog.suggested)
    }

    @Test func recentComesFirstAndLeavesActionsOut() {
        let rows = rows(recent: ["terminal-1", "new-terminal", "gone"])
        #expect(rows.first == .label(title: "Recent", count: nil))
        #expect(rows.compactMap(\.result?.id).first == "terminal-1")
        #expect(rows.compactMap(\.result?.id).filter { $0 == "new-terminal" }.count == 1)
    }

    @Test func aQueryListsGroupsInOrderWithTheirCounts() {
        let rows = rows(.all, "e")
        let labels = rows.compactMap { row -> String? in
            if case .label(let title, let count?) = row { "\(title) \(count)" } else { nil }
        }
        #expect(labels.first?.hasPrefix("Actions") == true)
        #expect(labels.contains { $0.hasPrefix("Terminals") })
        #expect(PaletteRows.resultCount(rows) >= rows.compactMap(\.result).count)
    }

    @Test func unscopedGroupsAreCutToFiveButTheirScopeListsAll() {
        let many = WorkspaceState(terminals: (1...9).map { TerminalSummary(id: $0, title: "zq\($0)") })
        let all = CommandCatalog.commands(for: many)
        let cut = PaletteRows.rows(scope: .all, query: "zq", commands: all)
        #expect(cut.compactMap(\.result).count == 5)
        #expect(PaletteRows.resultCount(cut) == 9)
        #expect(PaletteRows.rows(scope: .terminals, query: "zq", commands: all).compactMap(\.result).count == 9)
    }

    @Test func terminalsByWhoNeedsYouFirst() {
        let state = WorkspaceState(terminals: [
            TerminalSummary(id: 1, title: "a", activity: .idle), TerminalSummary(id: 2, title: "b", activity: .needsInput),
            TerminalSummary(id: 3, title: "c", activity: .working),
        ])
        let rows = PaletteRows.rows(scope: .terminals, query: "", commands: CommandCatalog.commands(for: state))
        #expect(rows.compactMap { if case .label(let t, _) = $0 { t } else { nil } } == ["Needs input", "Working", "Idle"])
    }

    @Test func actionsByTheirGroups() {
        let rows = rows(.actions)
        #expect(rows.first == .label(title: "Terminal", count: nil))
        #expect(rows.contains(.label(title: "View", count: nil)))
    }

    @Test func emptyScopesSayWhy() {
        #expect(rows(.commits) == [.note("Nothing here yet")])
        #expect(rows(.settings) == [.note("Type to find a setting")])
        #expect(rows(.all, "zzzqq") == [.note("Nothing matches")])
    }

    @Test func selectionStopsAtTheEndsAndSurvivesRowsChanging() {
        let rows = rows(.terminals)
        let ids = rows.compactMap(\.result?.id)
        #expect(PaletteSelection.current(chosen: nil, in: rows) == ids.first)
        #expect(PaletteSelection.step(-1, from: nil, in: rows) == ids.first)
        #expect(PaletteSelection.step(1, from: ids.last, in: rows) == ids.last)
        #expect(PaletteSelection.current(chosen: "vanished", in: rows) == ids.first)
    }

    @Test func footerKeysFollowTheScope() {
        #expect(PaletteFooter.hints(scope: .all).map(\.label) == ["move", "open", "scope"])
        #expect(PaletteFooter.hints(scope: .terminals).map(\.keys) == ["↩", "⌘⌫"])
        #expect(PaletteFooter.trailing(scope: .all, query: "", rows: []) == .actionsHint)
        #expect(PaletteFooter.trailing(scope: .all, query: "x", rows: rows(.all, "e")) != .actionsHint)
    }
}

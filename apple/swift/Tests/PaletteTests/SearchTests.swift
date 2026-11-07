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

    @Test func consecutiveBeatsSplit() throws {
        let run = try #require(score("new", "New Terminal"))
        let split = try #require(score("new", "Nice Extra Window"))
        #expect(run > split)
    }

    @Test func lettersInOrderAcrossWordStarts() {
        #expect(score("sp", "sidebar polish") != nil)
        #expect(score("shm", "theme shape metrics") != nil)
        #expect(score("ts", "Terminals") == nil)  // after a gap a letter has to start a word, as in core
        #expect(score("zq", "Terminals") == nil)
    }

    @Test func rangesMarkTheMatchedCharacters() throws {
        let match = try #require(PaletteSearch.fuzzy("hsid", in: "Hide Sidebar"))
        #expect(match.ranges == [0..<1, 5..<8])
        let whole = try #require(PaletteSearch.fuzzy("new", in: "New Terminal"))
        #expect(whole.ranges == [0..<3])
    }

    @Test func everyWordMustBeFoundAndSubtitleCountsLess() throws {
        #expect(PaletteSearch.score("go api", title: "Go to build", other: ["~/api · main"]) != nil)
        #expect(PaletteSearch.score("go zzz", title: "Go to build", other: ["~/api"]) == nil)
        let inTitle = try #require(PaletteSearch.score("api", title: "api server"))
        let inOther = try #require(PaletteSearch.score("api", title: "Go to build", other: ["api"]))
        #expect(inTitle.score > inOther.score)
        #expect(inOther.ranges.isEmpty)
    }

    @Test func caseDoesNotMatterAndUnicodeIsByCharacter() throws {
        #expect(score("NEW", "new terminal") != nil)
        let match = try #require(PaletteSearch.fuzzy("é", in: "Café Étoile"))
        #expect(match.ranges.first == 5..<6)
    }

    @Test func ranksAreStableForEqualScores() {
        let commands = (0..<4).map { Command(id: "c\($0)", title: "Same", category: .view, symbol: "x") }
        #expect(PaletteSearch.results(for: "same", in: commands).map(\.id) == ["c0", "c1", "c2", "c3"])
    }

    @Test func bestMatchComesFirst() {
        let state = sample()
        let found = PaletteSearch.results(for: "side", in: CommandCatalog.commands(for: state))
        #expect(found.first?.command.id == "toggle-sidebar")
    }

    @Test func emptyQueryGroupsActionsBeforeTerminals() {
        let all = PaletteSearch.results(for: "", in: CommandCatalog.commands(for: sample()))
        let groups = all.map(\.command.group.rawValue)
        #expect(groups == groups.sorted())
        #expect(all.first?.command.id == "new-terminal")
    }

    @Test func prefixesNarrowTheScope() {
        let commands = CommandCatalog.commands(for: sample())
        let terminals = PaletteSearch.results(for: "@ ", in: commands)
        #expect(!terminals.isEmpty && terminals.allSatisfy { $0.command.group == .terminals })
        let actions = PaletteSearch.results(for: ">new", in: commands)
        #expect(actions.map(\.id) == ["new-terminal"])
        #expect(PaletteScope.parse("@api").query == "api")
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

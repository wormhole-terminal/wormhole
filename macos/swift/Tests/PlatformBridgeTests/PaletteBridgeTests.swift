import PaletteModel
import Testing
import WormholeBindings

@testable import PlatformBridge

/// Core's search behind the palette's seam: the cases the Swift port of it used to carry, now against the real thing.
@Suite struct PaletteBridgeTests {
    private func rank(_ query: String, _ titles: [String], other: [[String]] = []) -> [PaletteRanked] {
        let candidates = titles.enumerated().map { PaletteCandidate(title: $1, other: other.indices.contains($0) ? other[$0] : []) }
        return CorePaletteMatcher().rank(query, candidates)
    }

    @Test func aPrefixBeatsAWordStartBeatsScattered() {
        let ranked = rank("sid", ["Inside out", "Hide Sidebar", "Sidebar polish", "nope"])
        #expect(ranked.map(\.index) == [2, 1, 0])
    }

    @Test func lettersInOrderAcrossWordStartsButNotAnyOrder() {
        #expect(!rank("sp", ["sidebar polish"]).isEmpty)
        #expect(!rank("shm", ["theme shape metrics"]).isEmpty)
        #expect(rank("ts", ["Terminals"]).isEmpty)
    }

    @Test func rangesMarkTheMatchedCharacters() {
        #expect(rank("hsid", ["Hide Sidebar"]).first?.match.ranges == [0..<1, 5..<8])
    }

    @Test func marksAreCharactersWhereCoreMarksBytes() {
        // "É" is two bytes, so the word starts at byte 8 and character 7.
        #expect(rank("sid", ["Éclair Sidebar"]).first?.match.ranges == [7..<10])
        #expect(rank("b", ["Ünï b"]).first?.match.ranges == [4..<5])
    }

    @Test func everyWordMustBeFoundAndOtherTextCountsLessAndIsNotMarked() throws {
        #expect(rank("go api", ["Go to build"], other: [["~/api · main"]]).count == 1)
        #expect(rank("go zzz", ["Go to build"], other: [["~/api"]]).isEmpty)
        let ranked = rank("api", ["Go to build", "api server"], other: [["api"], []])
        #expect(ranked.map(\.index) == [1, 0])
        #expect(ranked[1].match.ranges.isEmpty)
    }

    @Test func equalScoresKeepTheirOrderAndAnEmptyQueryFindsEverything() {
        #expect(rank("", ["b", "a"]).map(\.index) == [0, 1])
        #expect(rank("x", ["x1", "x2", "x3"]).map(\.index) == [0, 1, 2])
    }

    @Test func theScopeRowReadsTheSameInSwiftAndInCore() {
        let core = paletteScopes()
        #expect(core.map(\.title) == PaletteScope.row.map(\.title))
        #expect(core.map { $0.prefix.flatMap(\.first) } == PaletteScope.row.map(\.prefix))
        #expect(core.map(\.placeholder) == PaletteScope.row.map(\.placeholder))
        for input in ["@diff", "> new tab", "~", "#fix", ",theme", "theme", "", "/style.rs", "a@b", "  >  x"] {
            let parsed = paletteParse(input: input)
            let swift = PaletteScope.parse(input)
            #expect(parsed.query == swift.query, "\(input)")
            #expect(core.firstIndex { $0.scope == parsed.scope } == PaletteScope.row.firstIndex(of: swift.scope), "\(input)")
        }
        for (index, scope) in PaletteScope.row.enumerated() {
            let next = paletteStepScope(scope: paletteScopes()[index].scope, forward: true)
            #expect(core.firstIndex { $0.scope == next } == PaletteScope.row.firstIndex(of: scope.step(forward: true)))
        }
        #expect(Int(paletteGroupMax()) == PaletteGroup.unscopedMax)
        #expect(paletteGroupOrder() == ["Settings", "Actions", "Terminals", "Folders", "Commits"])
    }
}

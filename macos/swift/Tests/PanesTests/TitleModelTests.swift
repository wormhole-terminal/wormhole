import Testing

import PanesModel

@Suite struct TitleModelTests {
    /// One point per character, as a monospace font would measure.
    func chars(_: PathFace, _ text: String) -> Double { Double(text.count) }
    /// What the core lists for `~/a/bb/ccc` (`TitleSnapshot.pathSteps`), widest first, the last folder uncut.
    func steps(_ lastMax: Double = 100, last: String = "ccc") -> [PathStep] {
        PathSteps.measured(
            [PathText(parents: "~/a/bb/", last: last), PathText(parents: "~/…/bb/", last: last), PathText(parents: "~/…/", last: last)],
            lastMax: lastMax, measure: chars)
    }

    @Test func theCoresPathStepsAreMeasuredAsTheyCome() {
        #expect(steps().map(\.width) == [10, 10, 7])
        #expect(steps()[1].text == PathText(parents: "~/…/bb/", last: "ccc"))
    }

    @Test func aLongLastFolderIsCutInItsOwnMiddle() {
        #expect(steps(8, last: "abcdefghijklmnop").last?.text.last == "abcd…nop")
    }

    @Test func elidingKeepsTheExtensionAndTheEndOfAFolder() {
        #expect(Elide.middle("session_store_adapter.rs", 20) == "session_…_adapter.rs")
        #expect(Elide.middle("abcdefgh.rs", 9) == "abc…gh.rs")
        #expect(Elide.middle("short", 9) == "short" && Elide.middle("abcdef", 1) == "…")
        #expect(Elide.front("/Users/ben/Projects/adapters/session", 20) == "…/adapters/session")
        #expect(Elide.fit("abcdefghij", max: 6, measure: { Double($0.count) }, shorten: Elide.middle) == "abc…ij")
    }

    @Test func theMenuListsTheFoldersDownToTheVolume() {
        func names(_ path: String) -> [String] { PathChain.of(path, bootVolume: "Macintosh HD").map(\.name) }
        #expect(names("/Users/ben/Projects/tube") == ["tube", "Projects", "ben", "Users", "Macintosh HD"])
        #expect(PathChain.of("/Users/ben/Projects", bootVolume: "X")[1].path == "/Users/ben")
        #expect(names("/") == ["Macintosh HD"] && names("/tmp") == ["tmp", "Macintosh HD"])
        #expect(names("/Volumes/Work/clients/acme") == ["acme", "clients", "Work"])
        #expect(names("/Volumes") == ["Volumes", "Macintosh HD"] && names("a/b") == ["b", "a"] && names("").isEmpty)
    }

    @Test func theRowArrangementKeepsOneSpaceAndPutsAShownPartBackWhereItWas() {
        var row = TitleRow.default
        #expect(row.setShown(.ports, false) && !row.shows(.ports) && !row.setShown(.space, false))
        #expect(row.setShown(.ports, true) && row.parts == TitlePart.allCases)
        #expect(TitleRow([.terminal, .folder, .terminal]).parts == [.terminal, .space, .folder])
        row.move(from: 0, to: 4)
        #expect(row.parts.first == .branch && !row.isDefault)
    }

    @Test func withNoCoreToAskEveryPartKeepsItsNaturalWidth() {
        var w = RowWants()
        (w.path, w.words, w.name, w.group) = ([30, 20], 16, 40, 30)
        w.chip = ChipWidth(natural: 40, icon: 20)
        let fit = NaturalFit().fit(avail: 10, w)
        #expect(fit.path == .step(0) && fit.words && fit.chip == 40 && !fit.folded && fit.name == 40)
        #expect(NaturalFit().fit(avail: 100, RowWants()).path == .none)
    }

    @Test func theExtrasSayWhatTheyMean() {
        #expect(BranchStatus(dirty: true, ahead: 2, behind: 1, upstream: "origin/main").words
            == "Uncommitted changes, 2 commits ahead of origin/main, 1 commit behind origin/main")
        #expect(BranchStatus(ahead: 1).words == "1 commit ahead of")
        #expect(Ports.chips([3000, 5173, 8080, 9229]).more == 2 && Ports.chips([3000]).shown == [3000])
        #expect(Ports.label(3000) == "localhost:3000" && Ports.url(3000)?.absoluteString == "http://localhost:3000")
        #expect(Ports.moreWords([8080, 9229]) == "2 more listening: 8080, 9229")
    }

    /// Stepping, numbering and search are the core's (`WorkspaceBridgeTests` ask it); the list of sets is assembled here.
    @Test func tabSetsAreTheGroupsThenTheLooseTerminals() {
        let group = TabSet(groupID: 1, label: "A", members: [0])
        #expect(TabSets.of(groups: [group], loose: [4]).last?.label == "Ungrouped")
        #expect(TabSets.of(groups: [group], loose: []) == [group], "no loose terminal, no Ungrouped set")
    }
}

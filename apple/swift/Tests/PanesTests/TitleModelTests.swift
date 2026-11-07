import Testing

import PanesModel

@Suite struct TitleModelTests {
    /// One point per character, as a monospace font would measure.
    func chars(_: PathFace, _ text: String) -> Double { Double(text.count) }
    func texts(_ shown: String, _ lastMax: Double = 100) -> [String] {
        PathSteps.of(shown, lastMax: lastMax, measure: chars).map(\.text.full)
    }

    @Test func aPathLosesItsMiddleFromTheStartSide() {
        #expect(texts("~/Projects/misc/wormhole/Icssaddons/Tube") == [
            "~/Projects/misc/wormhole/Icssaddons/Tube", "~/…/misc/wormhole/Icssaddons/Tube",
            "~/…/wormhole/Icssaddons/Tube", "~/…/Icssaddons/Tube", "~/…/Tube",
        ])
        #expect(texts("~/Tube") == ["~/Tube"] && texts("~") == ["~"] && texts("/") == ["/"])
        #expect(texts("~/Projects/Tube") == ["~/Projects/Tube", "~/…/Tube"])
        #expect(texts("/Volumes/Work/clients/acme/api") == [
            "/Volumes/Work/clients/acme/api", "/Volumes/…/clients/acme/api", "/Volumes/…/acme/api", "/Volumes/…/api",
        ])
        let steps = PathSteps.of("~/a/bb/ccc", lastMax: 100, measure: chars)
        #expect(steps[1].text == PathText(parents: "~/…/bb/", last: "ccc") && steps[1].width == 10)
    }

    @Test func aLongLastFolderIsCutInItsOwnMiddle() {
        #expect(texts("~/a/abcdefghijklmnop", 8).last == "~/…/abcd…nop")
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
        #expect(row.ends.left == [.folder, .branch, .branchStatus, .terminal, .terminalStatus])
        #expect(row.setShown(.ports, true) && row.parts == TitlePart.allCases)
        #expect(TitleRow([.terminal, .folder, .terminal]).parts == [.terminal, .space, .folder])
        row.move(from: 0, to: 4)
        #expect(row.parts.first == .branch && !row.isDefault)
    }

    @Test func aChevronSeparatesTwoCrumbsAndAChipPartsThemWithoutOne() {
        #expect(chevrons(for: [.folder, .branch, .branchStatus, .terminal, .terminalStatus]) == [false, true, false, true, false])
        #expect(chevrons(for: [.folder, .ports, .terminal]) == [false, false, false])
        #expect(chevrons(for: [.openIn, .splits]) == [false, false])
    }

    private func wants(path: [Double] = [30, 20, 10], words: Double = 16, name: Double = 40) -> RowWants {
        var w = RowWants()
        (w.path, w.words, w.name, w.nameMin, w.fixed, w.group, w.folded) = (path, words, name, 12, 20, 30, 10)
        w.chip = ChipWidth(natural: 40, icon: 20)
        w.extras = Extras().with(.pullRequest, 20).with(.ports, 30).with(.gauge, 25).with(.branchStatus, 12)
        w.portsNarrower = [18]
        return w
    }

    @Test func aRoomyRowDrawsEverythingAndANarrowOneGivesWayInOrder() {
        let all = wants()
        // fixed 20 + path 30 + words 16 + extras 87 + chip 40 + group 30, and the name's 40.
        #expect(fitRow(avail: 263, all).path == .step(0))
        let shorter = fitRow(avail: 253, all)
        #expect(shorter.path == .step(1) && shorter.words && shorter.extras.shows(.pullRequest))
        #expect(fitRow(avail: 243, all).path == .step(2))
        let noWords = fitRow(avail: 220, all)
        #expect(!noWords.words && !noWords.extras.shows(.pullRequest) && noWords.extras.shows(.ports) && noWords.chip == 40)
        let noExtras = fitRow(avail: 150, all)
        #expect(Extra.allCases.allSatisfy { !noExtras.extras.shows($0) } && noExtras.chip == 40 && !noExtras.folded)
        let glyphOnly = fitRow(avail: 120, all)
        #expect(glyphOnly.chip == 20 && !glyphOnly.folded)
        let folded = fitRow(avail: 100, all)
        #expect(folded.folded && folded.chip == 20 && folded.name == 40)
        let tiny = fitRow(avail: 50, all)
        #expect(tiny.folded && tiny.path == .icon && tiny.name == 0)
    }

    @Test func thePortsLoseAChipBeforeTheyGoAndThePathFoldsToItsIconLast() {
        var w = wants(path: [30])
        w.words = 0
        w.extras = Extras().with(.ports, 30)
        w.chip = nil
        let fit = fitRow(avail: 140, w)
        #expect(fit.portsStep == 1 && fit.extras.width(.ports) == 18)
        w.name = 200
        #expect(fitRow(avail: 70, w).path == .icon)
        #expect(fitRow(avail: 100, RowWants()).path == .none)
    }

    @Test func theExtrasSayWhatTheyMean() {
        #expect(BranchStatus(dirty: true, ahead: 2, behind: 1, upstream: "origin/main").words
            == "Uncommitted changes, 2 commits ahead of origin/main, 1 commit behind origin/main")
        #expect(BranchStatus(ahead: 1).words == "1 commit ahead of")
        #expect(Ports.chips([3000, 5173, 8080, 9229]).more == 2 && Ports.chips([3000]).shown == [3000])
        #expect(Ports.label(3000) == "localhost:3000" && Ports.url(3000)?.absoluteString == "http://localhost:3000")
        #expect(Ports.moreWords([8080, 9229]) == "2 more listening: 8080, 9229")
    }

    @Test func tabSetsStepWrapAndSearch() {
        let sets = [
            TabSet(groupID: "a", label: "A", members: [0]), TabSet(groupID: "d", label: "Docs", members: []),
            TabSet(groupID: "b", label: "B", members: [1, 2]), TabSet(groupID: nil, label: TabSets.ungrouped, members: [3]),
        ]
        #expect(TabSets.step(sets, from: 0, forward: true) == 2 && TabSets.step(sets, from: 3, forward: true) == 0)
        #expect(TabSets.step(sets, from: 0, forward: false) == 3)
        #expect(TabSets.step([sets[0]], from: 0, forward: true) == nil)
        #expect(TabSets.current(sets, containing: 2) == 2 && TabSets.current(sets, containing: 9) == nil)
        #expect(TabSets.numbered(sets, 4) == 3 && TabSets.numbered(sets, 5) == nil && TabSets.numbered(sets, 0) == nil)
        let names = ["zero", "docs reader", "two", "three"]
        #expect(TabSets.search(sets, "doc", name: { names[$0] }) == [.set(1), .terminal(1, set: 2)])
        #expect(TabSets.search(sets, "  ", name: { names[$0] }).isEmpty)
        #expect(TabSets.of(groups: [sets[0]], loose: [4]).last?.label == "Ungrouped")
    }
}

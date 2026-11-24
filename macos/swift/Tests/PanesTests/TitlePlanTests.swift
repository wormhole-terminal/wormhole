import Testing

@testable import PanesModel
import WorkspaceModel

/// One point per character, as a monospace font would measure.
struct CharWidth: TextMeasuring {
    func width(_ text: String, _ face: TitleFace) -> Double { Double(text.count) }
}

@Suite struct TitlePlanTests {
    private func facts(_ row: TitleRow = .default) -> TitleFacts {
        TitleFacts(
            folder: "/Users/ben/Projects/tube", shownFolder: "~/Projects/tube", label: "claude", words: "working · 4m",
            branch: .some("main"), branchStatus: BranchStatus(dirty: true), gauge: GaugeFact(model: "Opus", context: 41),
            ports: [3000, 5173, 8080], pullRequest: PullRequestFact(number: 142, checks: .passing), row: row)
    }

    @Test func crumbsStandAGapApartAndButtonsFlush() {
        #expect(TitleLayout.gap(before: nil, .folder) == TitleMetrics.crumbPad)
        #expect(TitleLayout.gap(before: nil, .splits) == 0)
        #expect(TitleLayout.gap(before: .folder, .branch) == TitleMetrics.crumbGap)
        #expect(TitleLayout.gap(before: .pullRequest, .openIn) == TitleMetrics.rowGap)
        #expect(TitleLayout.gap(before: .openIn, .splits) == TitleMetrics.afterChevron)
        #expect(TitleLayout.gap(before: .splits, .openIn) == 0)
        #expect(TitleLayout.gap(before: .splits, .terminal) == TitleMetrics.rowGap)
    }

    @Test func theBreadcrumbKeepsItsFixedWidth() {
        // Its padding at each end, the folder's icon, two chevrons with a gap on each side, and the tile with its gap.
        let before = 2 * TitleMetrics.crumbPad + TitleMetrics.glyph + 2 * (2 * TitleMetrics.crumbGap + TitleMetrics.glyph)
            + TitleMetrics.tile + TitleMetrics.crumbGap
        let ends = (
            left: TitleEnd([.folder, .branch, .branchStatus, .terminal, .terminalStatus], chevrons: [false, true, false, true, false]),
            right: TitleEnd([.agentGauge, .ports, .pullRequest, .openIn, .splits], chevrons: [false, false, false, false, false]))
        #expect(TitleLayout.fixedWidth(ends) == before)
    }

    @Test func theRowGivesTheCoreItsWantsAndDrawsWhatComesBack() {
        let arrangement = TitleArrangement(
            left: TitleEnd([.folder, .terminal], chevrons: [false, true]), right: TitleEnd([.splits], chevrons: [false]),
            paths: [PathText(parents: "~/Projects/", last: "tube"), PathText(parents: "~/…/", last: "tube")])
        let roomy = TitlePlanner.plan(facts(), arrangement, width: 2000, measure: CharWidth(), fitting: NaturalFit())
        #expect(roomy.steps.map(\.width) == [15, 8] && roomy.fit.path == .step(0) && !roomy.fit.folded)
        #expect(roomy.draws(.folder) && roomy.draws(.splits) && !roomy.draws(.ports))
        // A fit that folds the row: the buttons are one menu, and only the first of them is the menu.
        struct Folding: RowFitting {
            func fit(avail: Double, _ wants: RowWants) -> RowFit {
                var fit = NaturalFit().fit(avail: avail, wants)
                fit.folded = true
                return fit
            }
        }
        let narrow = TitlePlanner.plan(facts(), arrangement, width: 100, measure: CharWidth(), fitting: Folding())
        #expect(narrow.fit.folded && narrow.isDrawn(.splits))
    }

    @Test func thePortsLoseAChipBeforeTheyGo() {
        var f = facts()
        f.row = TitleRow([.ports, .space])
        f.ports = [3000, 5173, 8080]
        let both = TitlePlanner.extras(f, CharWidth(), modelWidth: 4)
        #expect(both.1.count == 1 && both.1[0] < (both.0.width(.ports) ?? 0))
    }

    @Test func aRoomyRowGivesTheCutTextsTheRoomItHasLeft() {
        var long = facts()
        long.branch = .some(String(repeating: "b", count: 200))
        long.label = String(repeating: "n", count: 300)
        (long.branchStatus, long.gauge, long.ports, long.pullRequest) = (nil, nil, [], nil)
        let arrangement = TitleArrangement(
            left: TitleEnd([.folder, .branch, .terminal], chevrons: [false, true, true]), right: TitleEnd([.splits], chevrons: [false]),
            paths: [PathText(parents: "~/Projects/", last: "tube")])
        // Their guaranteed shares keep the order of giving way; with the row's width counted they are cut only where the room ends.
        let tight = TitlePlanner.plan(long, arrangement, width: 400, measure: CharWidth(), fitting: NaturalFit())
        #expect(tight.fit.chip == TitleMetrics.chipMax && tight.fit.name == TitleMetrics.nameMax)
        let roomy = TitlePlanner.plan(long, arrangement, width: 2000, measure: CharWidth(), fitting: NaturalFit())
        #expect(roomy.fit.chip == 200 + TitleMetrics.chipChrome && roomy.fit.name == 300)
        // Room for part of it: what the row has left is shared out, and nothing goes past it.
        let some = TitlePlanner.plan(long, arrangement, width: 700, measure: CharWidth(), fitting: NaturalFit())
        let used = TitleLayout.fixedWidth(arrangement.ends) + TitleMetrics.crumbGap + 15 + (some.fit.chip ?? 0) + TitleMetrics.rowGap + TitleMetrics.splitsWidth + some.fit.name
        #expect((some.fit.chip ?? 0) > TitleMetrics.chipMax && used <= 700)
    }

    @Test func aPullRequestAndAGaugeSpeak() {
        #expect(PullRequestFact(number: 7, state: .merged, checks: .failing).openChecks == nil)
        #expect(GaugeFact(model: "Opus 4.5", context: 41).words == "Opus 4.5, 41% of the context window used")
        #expect(GaugeFact(context: 80).warns && GaugeFact(context: 79).warns == false)
    }
}

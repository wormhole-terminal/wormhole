import Testing

@testable import PanesModel

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

    @Test func theDefaultBreadcrumbKeepsItsFixedWidth() {
        // Its padding at each end, the folder's icon, two chevrons with a gap on each side, and the tile with its gap.
        let before = 2 * TitleMetrics.crumbPad + TitleMetrics.glyph + 2 * (2 * TitleMetrics.crumbGap + TitleMetrics.glyph)
            + TitleMetrics.tile + TitleMetrics.crumbGap
        #expect(TitleLayout.fixedWidth(TitleLayout.ends(facts())) == before)
        // An end of buttons alone takes nothing of its own.
        let only = TitleFacts(label: "zsh", row: TitleRow([.terminal, .space, .openIn, .splits]))
        #expect(only.row.shows(.openIn) && TitleLayout.has(only, .openIn) == false)
    }

    @Test func aPartWithNothingToShowIsLeftOutAndNoChevronComesAndGoesWithRoom() {
        let plain = TitleFacts(label: "zsh")
        let ends = TitleLayout.ends(plain)
        #expect(ends.left.parts == [.terminal] && ends.right.parts == [.splits])
        let full = TitleLayout.ends(facts())
        #expect(full.left.parts == [.folder, .branch, .branchStatus, .terminal, .terminalStatus])
        #expect(full.left.chevrons == [false, true, false, true, false])
        #expect(full.right.parts == [.agentGauge, .ports, .pullRequest, .openIn, .splits])
    }

    @Test func aWideRowDrawsEverythingAndANarrowOneGivesWayInOrder() {
        let roomy = TitlePlanner.plan(facts(), width: 2000, measure: CharWidth())
        #expect(roomy.fit.path == .step(0) && roomy.fit.words && !roomy.fit.folded)
        #expect(Extra.allCases.allSatisfy { roomy.fit.extras.shows($0) })
        let narrow = TitlePlanner.plan(facts(), width: 330, measure: CharWidth())
        #expect(!narrow.fit.words && !narrow.fit.extras.shows(.pullRequest))
        let tiny = TitlePlanner.plan(facts(), width: 140, measure: CharWidth())
        #expect(tiny.fit.folded && tiny.fit.path == .icon && Extra.allCases.allSatisfy { !tiny.fit.extras.shows($0) })
        #expect(tiny.fit.chip == TitleMetrics.chipIcon || tiny.fit.chip == nil)
    }

    @Test func thePortsLoseAChipBeforeTheyGo() {
        var f = facts()
        f.row = TitleRow([.ports, .space])
        f.ports = [3000, 5173, 8080]
        let both = TitlePlanner.extras(f, CharWidth())
        #expect(both.1.count == 1 && both.1[0] < (both.0.width(.ports) ?? 0))
    }

    @Test func aPullRequestAndAGaugeSpeak() {
        #expect(PullRequestFact(number: 142, draft: true, checks: .failing).words == "Pull request #142, open, draft, checks failing")
        #expect(PullRequestFact(number: 7, state: .merged, checks: .failing).words == "Pull request #7, merged")
        #expect(PullRequestFact(number: 7, state: .merged, checks: .failing).openChecks == nil)
        #expect(GaugeFact(model: "Opus 4.5", context: 41).words == "Opus 4.5, 41% of the context window used")
        #expect(GaugeFact(context: 80).warns && GaugeFact(context: 79).warns == false)
    }
}

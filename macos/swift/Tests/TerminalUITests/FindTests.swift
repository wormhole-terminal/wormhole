import Testing

@testable import TerminalUI

@MainActor @Suite struct FindTests {
    private func settle() async { try? await Task.sleep(for: .milliseconds(40)) }

    private func open(_ text: String = "Compiling") async -> (FindModel, MockSearching) {
        let searching = MockSearching()
        let model = FindModel(searcher: searching)
        model.open(seed: text)
        await settle()
        return (model, searching)
    }

    @Test func aSearchStartsAtTheMatchNearestTheBottomAndRevealsIt() async {
        let (model, searching) = await open()
        #expect(model.outcome.count == 2)
        #expect(model.current == 1)
        #expect(searching.revealed == [1])
        #expect(model.countText == "2 of 2")
    }

    @Test func stepsGoRoundTheEnds() async {
        let (model, _) = await open()
        model.step(1)
        #expect(model.current == 0)
        model.step(-1)
        #expect(model.current == 1)
        model.step(-1)
        #expect(model.current == 0)
    }

    @Test func nothingTypedOpensTheBarInsteadOfStepping() async {
        let model = FindModel(searcher: MockSearching())
        model.step(1)
        #expect(model.isOpen)
        #expect(model.countText == "")
        let before = model.focusRequest
        model.step(1)
        #expect(model.focusRequest == before + 1, "an empty query asks for the keyboard again")
    }

    @Test func theCountSaysNoResultsAndBadPatterns() async {
        let (model, _) = await open("zzz")
        #expect(model.countText == "No results")
        model.toggle(.regex)
        model.setText("(")
        await settle()
        #expect(model.countText == "Not a valid pattern")
        #expect(model.countIsError)
    }

    @Test func noResultsWaitsForTheSearchSoTypingDoesNotFlashIt() async {
        let model = FindModel(searcher: MockSearching())
        model.open(seed: "zzz")
        #expect(model.isSearching)
        #expect(model.countText == "")
    }

    @Test func aCappedSearchReadsWithAPlus() {
        let model = FindModel(searcher: MockSearching())
        #expect(model.countText == "")
        // Through the same text the bar shows: 10,000+.
        #expect(L10n.findCount("1", "10,000+") == "1 of 10,000+")
    }

    @Test func optionsToggleAndSearchAgain() async {
        let (model, _) = await open("compiling")
        #expect(model.outcome.count == 2, "case is ignored by default")
        model.toggle(.matchCase)
        await settle()
        #expect(model.options.matchCase)
        #expect(model.outcome.count == 0)
    }

    @Test func theSelectionsFirstLineBecomesTheQuery() async {
        let searching = MockSearching(selection: "\n  \nwarning: unused import\nsecond")
        let model = FindModel(searcher: searching)
        model.useSelectionForFind()
        await settle()
        #expect(model.text == "warning: unused import")
        #expect(model.outcome.count == 1)
    }

    @Test func closingClearsWhatWasFound() async {
        let (model, _) = await open()
        model.close()
        #expect(!model.isOpen && model.outcome.count == 0 && model.current == nil && model.ticks.isEmpty)
    }

    @Test(.timeLimit(.minutes(1))) func printingSearchesAgainWithoutMovingTheCurrentMatch() async {
        let (model, searching) = await open()
        model.step(1)
        searching.lines.append("   Compiling more")
        searching.hasNewOutput = true
        // The search after the wait says when it landed: a fixed time is lost on a machine whose main actor is busy
        // with other suites (GitHub's runner took longer than 320 ms).
        await withCheckedContinuation { landed in
            model.onOutcome = {
                model.onOutcome = nil
                landed.resume()
            }
            model.outputChanged()
        }
        searching.hasNewOutput = false
        #expect(model.outcome.count == 3)
        #expect(model.current == 0, "the current match stays where it was")
    }
}

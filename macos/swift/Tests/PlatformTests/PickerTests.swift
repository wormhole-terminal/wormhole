import CoreGraphics
import PlatformModel
import Testing

/// The folder picker's rules (`view/picker/{keys,ghost,place}.rs`) and its model over a provider.
@Suite struct PickerTests {
    static let all = { () -> PickerFacts in
        var facts = PickerFacts()
        (facts.up, facts.upAtEnd, facts.descends, facts.ghost, facts.forgettable) = (true, true, true, true, true)
        return facts
    }()

    private func at(_ text: String, _ caret: Int) -> PickerField { PickerField(text: text, caret: caret) }

    @Test func theListKeysAreAlwaysThePickers() {
        let none = PickerFacts(), field = at("abc", 1)
        #expect(PickerKeys.decide(.up, field: field, facts: none) == .step(-1))
        #expect(PickerKeys.decide(.down, field: field, facts: none) == .step(1))
        #expect(PickerKeys.decide(.pageDown, field: field, facts: none) == .step(8))
        #expect(PickerKeys.decide(.enter, field: field, facts: none) == .pick)
        #expect(PickerKeys.decide(.escape, field: field, facts: none) == .close)
        #expect(PickerKeys.decide(.tab, field: field, facts: none) == .descend)
    }

    @Test func rightDescendsOnlyAtTheEndOfTheText() {
        #expect(PickerKeys.decide(.right, field: at("src", 3), facts: Self.all) == .descend)
        #expect(PickerKeys.decide(.right, field: at("src", 1), facts: Self.all) == .pass)
        #expect(PickerKeys.decide(.right, field: PickerField(text: "src", caret: 0, selected: 3), facts: Self.all) == .pass)
        var nothing = Self.all
        (nothing.descends, nothing.ghost) = (false, false)
        #expect(PickerKeys.decide(.right, field: at("src", 3), facts: nothing) == .pass)
    }

    @Test func leftGoesUpAtTheStartOrWhereTheTextEndsALevel() {
        #expect(PickerKeys.decide(.left, field: at("src", 0), facts: Self.all) == .up)
        #expect(PickerKeys.decide(.left, field: at("src/", 4), facts: Self.all) == .up)
        var flat = Self.all
        flat.upAtEnd = false
        #expect(PickerKeys.decide(.left, field: at("src", 3), facts: flat) == .pass)
        #expect(PickerKeys.decide(.forget, field: at("a", 1), facts: PickerFacts()) == .pass)
        #expect(PickerKeys.decide(.forget, field: at("a", 1), facts: Self.all) == .forget)
    }

    @Test func theGhostIsWhatTheCompletionAdds() {
        #expect(PickerKeys.ghost(typed: "pro", completion: "Projects/") == "jects/")
        #expect(PickerKeys.ghost(typed: "~/Pr", completion: "~/Projects/") == "ojects/")
        #expect(PickerKeys.ghost(typed: "tmnl", completion: "terminal/") == nil)
        #expect(PickerKeys.ghost(typed: "src/", completion: "src/") == nil && PickerKeys.ghost(typed: "", completion: "src/") == nil)
        #expect(PickerKeys.ghost(typed: "é", completion: "Éclair/") == "clair/")
    }

    // MARK: placement

    private let card = PickerCard(width: 300, chrome: 52, row: 28, maxRows: 8)
    private let pane = CGRect(x: 0, y: 0, width: 800, height: 600)

    @Test func itSitsAboveTheCursorsLineWithItsBottomAtTheLine() {
        let cursor = CGRect(x: 100, y: 400, width: 8, height: 18)
        let placed = PickerPlacement.place(pane: pane, cursor: cursor, card: card, wanted: 5)
        #expect(placed.above && placed.rows == 5)
        #expect(placed.rect.maxY == cursor.minY - PickerPlacement.gap && placed.rect.minX == 100)
    }

    @Test func aCursorOnTheFirstLineGoesBelow() {
        let cursor = CGRect(x: 100, y: 20, width: 8, height: 18)
        let placed = PickerPlacement.place(pane: pane, cursor: cursor, card: card, wanted: 5)
        #expect(!placed.above && placed.rect.minY == cursor.maxY + PickerPlacement.gap)
    }

    @Test func typingNeverMovesTheCardAcrossTheLine() {
        let cursor = CGRect(x: 100, y: 200, width: 8, height: 18)
        let sides = (1...8).map { PickerPlacement.place(pane: pane, cursor: cursor, card: card, wanted: $0).above }
        #expect(Set(sides).count == 1)
    }

    @Test func theCardStaysInsideTheNarrowestPane() {
        let small = CGRect(x: 0, y: 0, width: 200, height: 160)
        let placed = PickerPlacement.place(pane: small, cursor: CGRect(x: 190, y: 80, width: 8, height: 18), card: card, wanted: 8)
        #expect(placed.rect.minX >= PickerPlacement.margin && placed.rect.maxX <= small.maxX - PickerPlacement.margin)
        #expect(placed.rect.minY >= PickerPlacement.margin && placed.rect.maxY <= small.maxY - PickerPlacement.margin)
        #expect(placed.rows >= 1)
    }

    // MARK: the model

    private func model() -> PickerModel { PickerModel(context: PickerContext(cwd: "/Users/ben/Projects", home: "/Users/ben")) }

    @Test func aLateAnswerForOldTextIsDropped() async {
        var model = model()
        let first = model.type("p")
        let second = model.type("pr")
        let answer = await MockFolders.sample.answer(first.query)
        let stale = model.receive(answer, generation: first.generation)
        let current = model.receive(await MockFolders.sample.answer(second.query), generation: second.generation)
        #expect(!stale && current)
        #expect(model.answer.rows.map(\.label) == ["Projects"])
        #expect(model.selected?.label == "Projects" && model.ghost == "ojects/")
    }

    @Test func theSelectionKeepsItsRowAcrossAnswersAndStepsOverPickableRows() async {
        var model = model()
        let (query, generation) = model.type("")
        model.receive(PickerMerge.merge([await MockFolders.sample.answer(query)]), generation: generation)
        #expect(model.lines.contains(.header("Recent")))
        let first = model.selectedKey
        model.step(2)
        #expect(model.selectedKey != first)
        model.step(-100)
        #expect(model.selectedKey == first)
        model.step(100)
        #expect(model.selected?.label == "downloads".capitalized || model.selected?.section == 1)
    }

    @Test func keysTurnIntoOutcomes() async {
        var model = model()
        let (query, generation) = model.type("pro")
        model.receive(await MockFolders.sample.answer(query), generation: generation)
        let descend = model.apply(PickerKeys.decide(.tab, field: PickerField(text: "pro"), facts: model.facts))
        let pick = model.apply(.pick), close = model.apply(.close), pass = model.apply(.pass), up = model.apply(.up)
        #expect(descend == .setText("Projects/") && pick == .pick("Projects/"))
        #expect(close == .close && pass == .nothing && up == .setText(""))
    }

    @Test func mergingRanksRowsOfASectionTogetherAndListsARowOnce() {
        func answer(_ rows: [(String, Int)], section: String = "") -> PickerAnswer {
            var answer = PickerAnswer()
            answer.sections = [section]
            answer.rows = rows.map { var row = PickerRow(key: $0.0, label: $0.0); row.score = $0.1; return row }
            return answer
        }
        let merged = PickerMerge.merge([answer([("a", 1), ("b", 5)]), answer([("b", 9), ("c", 3)]), answer([("r", 1)], section: "Recent")])
        #expect(merged.rows.map(\.key) == ["b", "c", "a", "r"])
        #expect(merged.sections == ["", "Recent"] && merged.rows.last?.section == 1)
    }

    @Test func decorationsFillBadgesAndChevrons() async {
        var model = model()
        let (query, generation) = model.type("pro")
        model.receive(await MockFolders.sample.answer(query), generation: generation)
        model.decorate(await MockFolders.sample.decorate(model.answer.rows.map(\.key)))
        #expect(model.answer.rows[0].badge == "main" && model.answer.rows[0].chevron)
    }

    @Test @MainActor func theDriverMergesProvidersAsTheyAnswer() async throws {
        let driver = FolderPickerDriverProbe.make()
        driver.open()
        // The providers answer on their own threads: wait for what the test needs, not for a time that a busy machine overruns.
        #expect(await Self.settles { driver.model.lines.contains(.header("Recent")) })
        driver.typed("pro")
        #expect(await Self.settles { driver.model.answer.rows.map(\.label) == ["Projects"] })
        #expect(driver.key(.tab) && driver.model.text == "Projects/")
    }

    /// Polls `condition` (on the main actor) until it holds; false after ten seconds, long past any answer.
    @MainActor static func settles(_ condition: @MainActor () -> Bool) async -> Bool {
        for _ in 0..<1000 {
            if condition() { return true }
            try? await Task.sleep(for: .milliseconds(10))
        }
        return condition()
    }
}

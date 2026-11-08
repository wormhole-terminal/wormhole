import XCTest

@testable import PanelModel

final class ReviewAndSyntaxTests: XCTestCase {
    func testMessageNamesPlacesQuotesLinesAndStripsControls() throws {
        var review = Review()
        review.put(ReviewComment(path: "src/orders.rs", side: .new, line: 42, code: "    let total = 0;\u{1b}[31m", text: "Use a checked add.\nAnd test it."))
        review.put(ReviewComment(path: "src/api.rs", side: .old, line: 7, code: "", text: "Why removed?"))
        let message = try XCTUnwrap(review.message)
        XCTAssertEqual(
            message,
            """
            Review comments on your changes. Please address each one:

            1. src/orders.rs:42
               > let total = 0;[31m
               Use a checked add.
               And test it.

            2. src/api.rs:7 (removed line)
               Why removed?
            """)
        XCTAssertEqual(review.summary, "2 comments on 2 files")
    }

    func testPutReplacesOnTheSameLineAndEmptyRemoves() {
        var review = Review()
        let comment = ReviewComment(path: "a", side: .new, line: 1, code: "x", text: "one")
        review.put(comment)
        review.put(ReviewComment(path: "a", side: .new, line: 1, code: "x", text: "two"))
        XCTAssertEqual(review.comments.map(\.text), ["two"])
        XCTAssertEqual(review.summary, "1 comment")
        review.put(ReviewComment(path: "a", side: .new, line: 1, code: "", text: "  "))
        XCTAssertTrue(review.isEmpty)
        XCTAssertNil(review.message)
    }

    func testReorderMovesStepsAndStopsAtTheEnds() {
        var items = ["a", "b", "c", "d"]
        XCTAssertTrue(Reorder.move(&items, from: 0, to: 2))
        XCTAssertEqual(items, ["b", "c", "a", "d"])
        XCTAssertFalse(Reorder.move(&items, from: 1, to: 1))
        XCTAssertFalse(Reorder.move(&items, from: 4, to: 0))
        XCTAssertNil(Reorder.step(0, -1, count: 4))
        XCTAssertEqual(Reorder.step(1, 1, count: 4), 2)
        XCTAssertEqual(Reorder.edge(from: 0, over: 2), .below)
        XCTAssertEqual(Reorder.edge(from: 3, over: 1), .above)
        XCTAssertNil(Reorder.edge(from: 1, over: 1))
    }

    func testBlockCommentsAndTripleQuotesCarryAcrossLines() throws {
        let highlighter = LexerHighlighter()
        let rust = highlighter.highlight(path: "a.rs", lines: ["let a = 1; /* start", "still comment let", "end */ let b"])
        XCTAssertEqual(rust[1].map(\.key), ["comment"])
        XCTAssertEqual(rust[2].first?.key, "comment")
        XCTAssertTrue(rust[2].contains { $0.key == "keyword" }, "after the comment closes, code again")
        let python = highlighter.highlight(path: "a.py", lines: ["x = \"\"\"doc", "def not_code", "\"\"\" + y"])
        XCTAssertEqual(python[1].first?.key, "string")
        XCTAssertNil(highlighter.highlight(path: "a.unknown", lines: ["x"]).first?.first)
    }

    func testDiffHighlightingReadsOldAndNewSidesSeparately() throws {
        var file = SampleDiffs.sidebar
        DiffHighlighting.apply(to: &file, with: LexerHighlighter())
        let lines = file.hunks[0].lines
        XCTAssertTrue(lines.filter { $0.kind == .added }.contains { $0.spans.contains { $0.key == "keyword" } })
        XCTAssertEqual(lines.first { $0.kind == .context }?.spans.isEmpty, false, "context lines take the new side's colours")
        XCTAssertTrue(lines.filter { $0.kind == .removed }.contains { $0.spans.contains { $0.key == "keyword" } })
    }

    func testUsageWordsMatchTheGPUIApp() {
        let limit = PlanLimit(label: "5-hour limit", used: 25, resetsAt: 1_000 + 34 * 60)
        XCTAssertEqual(limit.shown(now: 1_000).words, "25% \u{b7} resets 34m")
        XCTAssertEqual(PlanLimit(label: "w", used: 50, resetsAt: 900).shown(now: 1_000).words, "Awaiting update")
        XCTAssertEqual(UsageText.duration(3 * 86_400 - 60), "3d")
        XCTAssertEqual(UsageText.duration(2 * 3600 + 5 * 60), "2h 5m")
        XCTAssertEqual(UsageText.updated(0, now: 59), "Updated just now")
        XCTAssertEqual(UsageText.updated(0, now: 180), "Updated 3m ago")
        XCTAssertEqual([0.5, 0.8, 1.0].map(PlanLimit.level(of:)), [.normal, .warn, .danger])
    }

    func testProjectNoticeWords() {
        var state = PanelSamples.project
        XCTAssertEqual(state.notice.title, "Trust this project?")
        state.status = .changed
        XCTAssertEqual(state.notice.text, "Review what it would run now and approve again.")
        state.commands = []
        XCTAssertTrue(state.notice.text.hasPrefix("Trusting it applies its settings"))
    }
}

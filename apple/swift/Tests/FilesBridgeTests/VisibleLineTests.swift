import Foundation
import PanelModel
import XCTest

@testable import FilesBridge

/// The core's rule for a line of code, through the FFI, on the lines that merged graphemes when Swift counted Characters: a
/// control, a tab or a bidi override followed by a combining mark. Every offset must stay inside the drawn text.
@MainActor
final class VisibleLineTests: XCTestCase {
    func testHostileLinesStayInsideTheirDrawnText() {
        FilesBridge.installRules()
        for original in ["# \u{1}\u{301}", "#\t\u{301}", "a\u{202E}\u{301}b", "\u{85}\u{301}\t\u{2028}\u{301}x"] {
            let spans = [SyntaxSpan(range: 0..<original.count, key: "comment"), SyntaxSpan(range: 1..<original.count + 9, key: "string")]
            let drawn = DrawnLine(original, spans: spans, tabWidth: 4, limit: 5_000)
            let text = AttributedString(drawn.text)
            XCTAssertTrue(drawn.spans.allSatisfy { $0.range.upperBound <= drawn.scalars }, original)
            for range in drawn.spans.map(\.range) + drawn.marks {
                // What the drawing does: scalar indices into the drawn text. A past-the-end index would trap here.
                _ = text.unicodeScalars.index(text.startIndex, offsetBy: range.lowerBound)
                _ = text.unicodeScalars.index(text.startIndex, offsetBy: range.upperBound)
            }
        }
        let shown = DrawnLine("if x\u{202E} \u{2066}{", spans: [], tabWidth: 4, limit: 5_000)
        XCTAssertEqual(shown.text, "if x<U+202E> <U+2066>{")
        XCTAssertEqual(shown.marks, [4..<12, 13..<21])
    }

    /// Git's problems are worded by the core (`git::problem`), the same sentences as the GPUI app's.
    func testGitProblemsAreWordedByTheCore() {
        FilesBridge.installRules()
        let missing = GitProblem(message: "git is not installed", tab: .history)
        XCTAssertEqual(missing.headline, "History needs git, and it isn't installed.")
        XCTAssertEqual(missing.detail, "xcode-select --install")
        XCTAssertNil(missing.copy)
        let owner = GitProblem(message: "fatal: detected dubious ownership in repository at '/srv/repo'\nmore", tab: .changes)
        XCTAssertEqual(owner.detail, "git config --global --add safe.directory '/srv/repo'")
        let quoted = GitProblem(message: "fatal: detected dubious ownership in repository at '/srv/it's'", tab: .changes)
        XCTAssertEqual(quoted.headline, "Couldn't read this repository.")
        XCTAssertEqual(quoted.copy, "fatal: detected dubious ownership in repository at '/srv/it's'")
    }
}

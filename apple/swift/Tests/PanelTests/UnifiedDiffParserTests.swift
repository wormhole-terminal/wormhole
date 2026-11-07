import XCTest

@testable import PanelModel

final class UnifiedDiffParserTests: XCTestCase {
    func testHunksKeepLineNumbersOnBothSides() throws {
        let file = try XCTUnwrap(UnifiedDiffParser.parse(SampleDiffs.sidebar).first)
        XCTAssertEqual(file.path, "crates/wormhole/src/sidebar/list.rs")
        XCTAssertEqual(file.hunks.count, 2)
        let first = file.hunks[0]
        XCTAssertEqual(first.range, "-41,8 +41,10")
        XCTAssertEqual(first.section, "impl SidebarList {")
        XCTAssertEqual(first.lines.first, DiffLine(kind: .context, oldNumber: 41, newNumber: 41, text: "    /// The rows in the order they are drawn."))
        let removed = try XCTUnwrap(first.lines.first { $0.kind == .removed })
        XCTAssertEqual([removed.oldNumber, removed.newNumber], [43, nil])
        let added = try XCTUnwrap(first.lines.first { $0.kind == .added })
        XCTAssertEqual([added.oldNumber, added.newNumber], [nil, 43])
    }

    func testHunkLineCountsMatchTheirHeaders() throws {
        for sample in [SampleDiffs.sidebar, SampleDiffs.settings, SampleDiffs.newFile, SampleDiffs.renamed] {
            for hunk in try XCTUnwrap(UnifiedDiffParser.parse(sample).first).hunks {
                let old = hunk.lines.count { $0.kind == .context || $0.kind == .removed }
                let new = hunk.lines.count { $0.kind == .context || $0.kind == .added }
                let counts = hunk.range.split { " ,-+".contains($0) }.compactMap { Int($0) }
                XCTAssertEqual([old, new], [counts[1], counts.count == 4 ? counts[3] : 1], hunk.range)
            }
        }
    }

    func testMissingFinalNewlineIsItsOwnLine() throws {
        let file = try XCTUnwrap(UnifiedDiffParser.parse(SampleDiffs.settings).first)
        let last = try XCTUnwrap(file.hunks.first?.lines.last)
        XCTAssertEqual(last.kind, .noNewline)
        XCTAssertEqual(last.text, "No newline at end of file")
        XCTAssertEqual(file.hunks[0].lines.count { $0.kind == .context }, 3)
    }

    func testNewFileHasNoOldPath() throws {
        let file = try XCTUnwrap(UnifiedDiffParser.parse(SampleDiffs.newFile).first)
        XCTAssertTrue(file.isNew)
        XCTAssertEqual(file.hunks.first?.added, 5)
        XCTAssertEqual(file.hunks.first?.removed, 0)
    }

    func testRenameKeepsBothPaths() throws {
        let file = try XCTUnwrap(UnifiedDiffParser.parse(SampleDiffs.renamed).first)
        XCTAssertTrue(file.isRenamed)
        XCTAssertEqual([file.oldPath, file.newPath], ["docs/panel.md", "docs/notes-panel.md"])
        // The blank context line arrives without its leading space.
        XCTAssertEqual(file.hunks.first?.lines[2], DiffLine(kind: .context, oldNumber: 2, newNumber: 2, text: ""))
    }

    func testBinaryFileHasNoHunks() throws {
        let file = try XCTUnwrap(UnifiedDiffParser.parse(SampleDiffs.binary).first)
        XCTAssertTrue(file.isBinary)
        XCTAssertTrue(file.hunks.isEmpty)
    }

    func testSeveralFilesInOneText() {
        let files = UnifiedDiffParser.parse(SampleDiffs.settings + "\n" + SampleDiffs.binary)
        XCTAssertEqual(files.map(\.path), ["crates/wormhole-core/src/config/settings.rs", "packaging/icon.png"])
    }

    func testRemovedLineThatLooksLikeAFileHeaderStaysInItsHunk() throws {
        let text = """
            diff --git a/x.sql b/x.sql
            --- a/x.sql
            +++ b/x.sql
            @@ -1,2 +1,1 @@
            --- a comment
             keep
            """
        let hunk = try XCTUnwrap(UnifiedDiffParser.parse(text).first?.hunks.first)
        XCTAssertEqual(hunk.lines.map(\.kind), [.removed, .context])
        XCTAssertEqual(hunk.lines.first?.text, "-- a comment")
    }

    func testPathWithSpacesAndQuotes() throws {
        let text = "diff --git \"a/my file.txt\" \"b/my file.txt\"\nnew file mode 100644\n"
        XCTAssertEqual(UnifiedDiffParser.parse(text).first?.path, "my file.txt")
    }

    func testEmptyAndGarbageInput() {
        XCTAssertTrue(UnifiedDiffParser.parse("").isEmpty)
        XCTAssertTrue(UnifiedDiffParser.parse("not a diff\n@@ nonsense\n").isEmpty)
    }

    func testMalformedHunkHeaderDoesNotCrash() {
        let text = "diff --git a/f b/f\n@@ -x +y @@\n+line\n"
        XCTAssertEqual(UnifiedDiffParser.parse(text).first?.hunks.count, 0)
    }
}

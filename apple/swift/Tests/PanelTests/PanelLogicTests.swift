import XCTest

@testable import PanelModel

final class PanelLogicTests: XCTestCase {
    func testAddTrimsAndIgnoresBlank() {
        var list = TaskList()
        XCTAssertNil(list.add("   \n"))
        XCTAssertEqual(list.add("  write tests ")?.text, "write tests")
    }

    func testRenameToNothingRemoves() throws {
        var list = TaskList()
        let one = try XCTUnwrap(list.add("one"))
        list.rename(one.id, to: " two ")
        XCTAssertEqual(list.items.map(\.text), ["two"])
        list.rename(one.id, to: "  ")
        XCTAssertTrue(list.items.isEmpty)
    }

    func testDoneStepsFoldPastFive() {
        var list = TaskList((0..<8).map { PanelTask(text: "s\($0)", done: $0 < 5) })
        XCTAssertEqual(list.rows(showDone: false).count, 8, "five done stay where they are")
        list = TaskList((0..<8).map { PanelTask(text: "s\($0)", done: $0 < 6) })
        XCTAssertEqual(list.rows(showDone: false).count, 3, "two open, then the fold")
        XCTAssertEqual(list.rows(showDone: false).last, .doneFold(6))
        XCTAssertEqual(list.rows(showDone: true).count, 9)
    }

    func testFileNameFolderAndRename() {
        let file = ChangedFile(path: "crates/wormhole/src/lib.rs", kind: .modified)
        XCTAssertEqual([file.name, file.folder], ["lib.rs", "crates/wormhole/src"])
        let moved = ChangedFile(path: "lib/a.rs", kind: .renamed, originalPath: "src/a.rs")
        XCTAssertNil(moved.renamedFrom)
        XCTAssertTrue(moved.moved)
        let renamed = ChangedFile(path: "src/new.rs", kind: .renamed, originalPath: "src/old.rs")
        XCTAssertEqual(renamed.renamedFrom, "old.rs")
        XCTAssertFalse(renamed.moved)
    }

    func testTooltipSecondLine() {
        XCTAssertEqual(ChangedFile(path: "a/b.rs", kind: .added, added: 2210, removed: 0).about, "Added · +2210")
        XCTAssertEqual(ChangedFile(path: "a/b.rs", kind: .modified, added: 128, removed: 40).about, "Modified · +128 \u{2212}40")
        XCTAssertEqual(ChangedFile(path: "a.png", kind: .modified, added: nil).about, "Modified · binary")
        XCTAssertEqual(ChangedFile(path: "a", kind: .modified, lfs: true).about, "Modified · LFS")
        XCTAssertEqual(
            ChangedFile(path: "lib/new.rs", kind: .renamed, added: 1, removed: 1, originalPath: "src/old.rs").about,
            "Renamed from src/old.rs · +1 \u{2212}1")
    }

    func testAgeWordsAndGrouping() {
        XCTAssertEqual([0, 59, 60, 3_599, 3_600, 86_400, 604_800, -5].map(Age.text(seconds:)), ["now", "now", "1m", "59m", "1h", "1d", "1w", "now"])
        XCTAssertEqual(CountText.grouped(1_234_567), "1,234,567")
        XCTAssertEqual(CountText.capped(1_200), "999+")
    }

    func testSectionsAndTree() {
        let files = [
            ChangedFile(path: "src/a/b/c.rs", kind: .modified), ChangedFile(path: "README.md", kind: .modified),
            ChangedFile(path: "new.txt", kind: .untracked),
        ]
        let flat = ChangeRows(tree: false).build(files)
        XCTAssertEqual(flat.filter { if case .band = $0 { true } else { false } }.count, 2)
        let tree = ChangeRows(tree: true, groupUntracked: false).build(files)
        XCTAssertTrue(tree.contains(.directory(group: .all, path: "src/a/b", label: "src/a/b", depth: 0, collapsed: false)), "a chain of single folders is one row")
        XCTAssertEqual(ChangeRows(tree: true).order(files).map(\.path), ["src/a/b/c.rs", "README.md", "new.txt"])
    }

    func testBigSectionStartsFoldedWithANote() {
        let files = (0..<201).map { ChangedFile(path: "build/f\($0).o", kind: .untracked) }
        let rows = ChangeRows().build(files)
        guard case let .band(_, count, folded, _) = rows[0], case let .foldNote(note) = rows[1] else { return XCTFail("\(rows)") }
        XCTAssertEqual(count, 201)
        XCTAssertTrue(folded)
        XCTAssertEqual(note, "201 of them are in build/, which isn't ignored. Folded because there are more than 200.")
    }

    func testSplitPairsRunsAndFillsTheRest() throws {
        let file = try XCTUnwrap(UnifiedDiffParser.parse(SampleDiffs.sidebar).first)
        let rows = DiffRows.split(file)
        let pairs = rows.compactMap { if case let .split(old, new) = $0 { (old?.kind, new?.kind) } else { nil } }
        XCTAssertTrue(pairs.contains { $0.0 == .removed && $0.1 == .added })
        XCTAssertTrue(pairs.contains { $0.0 == nil && $0.1 == .added }, "the longer run's rest meets fillers")
        XCTAssertEqual(DiffRows.numberDigits(file), 3)
    }

    func testNoNewlineIsAFlagNotARow() throws {
        let file = try XCTUnwrap(UnifiedDiffParser.parse(SampleDiffs.settings).first)
        let rows = DiffRows.unified(file)
        XCTAssertEqual(rows.count, 1 + file.hunks[0].lines.count - 1)
        guard case let .unified(last) = rows.last else { return XCTFail() }
        XCTAssertTrue(last.noNewline)
    }

    func testLexerNamesTheUsualRuns() throws {
        let rust = try XCTUnwrap(SyntaxLanguage.of(path: "a.rs"))
        let keys = SyntaxLexer.spans(in: "pub fn go(x: u8) -> Option<u8> { // hi", language: rust).map(\.key)
        XCTAssertEqual(keys.first, "keyword")
        XCTAssertTrue(keys.contains("function") && keys.contains("type") && keys.last == "comment")
        XCTAssertNil(SyntaxLanguage.of(path: "notes.unknownext"), "no grammar, no colours")
    }

    func testCommitStampAndMerge() {
        XCTAssertEqual(Signature(name: "a", time: 1_790_000_000, zone: 7200).stamp, "2026-09-21 16:13 +02:00")
        XCTAssertEqual(Commit(sha: "abcdef0123", subject: "s", author: Signature(name: "a", time: 0), parents: ["1", "2"]).isMerge, true)
    }
}

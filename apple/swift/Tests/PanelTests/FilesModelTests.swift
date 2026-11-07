import XCTest

@testable import FileIconTheme
@testable import PanelModel

final class FilesModelTests: XCTestCase {
    private func dir(_ entries: [(String, DiskEntry.Kind)], more: Int = 0) -> Directory {
        Directory(entries: entries.map { DiskEntry(name: $0.0, kind: $0.1) }, more: more)
    }

    func testClosedFolderShowsNothingBelowAndOpeningAsksForARead() {
        var tree = FileTree()
        tree.set("", .success(dir([("src", .folder), ("a.rs", .file), ("l", .link)])))
        XCTAssertEqual(tree.rows.map(\.label), ["src", "a.rs", "l"])
        XCTAssertTrue(tree.toggle("src"), "not read yet")
        XCTAssertEqual(tree.rows.map(\.kind).dropFirst().first, .reading)
        XCTAssertFalse(tree.toggle("l"), "a link never opens")
        XCTAssertFalse(tree.toggle("a.rs"))
        tree.set("src", .success(dir([("m.rs", .file)])))
        XCTAssertEqual(tree.rows.map(\.path), ["src", "src/m.rs", "a.rs", "l"])
        XCTAssertEqual(tree.wanted, ["", "src"])
    }

    func testNotesForEmptyMoreAndUnreadable() {
        var tree = FileTree()
        tree.set("", .success(dir([("e", .folder), ("x", .folder)], more: 7)))
        tree.toggle("e")
        tree.toggle("x")
        tree.set("e", .success(dir([])))
        tree.set("x", .failure(.denied))
        let kinds = tree.rows.map(\.kind)
        XCTAssertEqual(kinds, [.folder(open: true), .empty, .folder(open: true), .unreadable(.denied), .more(7, atLeast: false)])
    }

    func testAFolderGoneFromItsParentTakesWhatWasUnderItAlong() {
        var tree = FileTree()
        tree.set("", .success(dir([("a", .folder)])))
        tree.toggle("a")
        tree.set("a", .success(dir([("b", .folder)])))
        XCTAssertEqual(tree.rows.count, 2)
        XCTAssertTrue(tree.set("", .success(dir([]))), "pruning is a change")
        XCTAssertEqual(tree.rows.map(\.kind), [.empty])
        XCTAssertFalse(tree.set("", .success(dir([]))), "the same answer changes nothing")
    }

    func testRenamedFolderKeepsItsStateAndOpenness() {
        var tree = FileTree()
        tree.set("", .success(dir([("old", .folder)])))
        tree.toggle("old")
        tree.set("old", .success(dir([("f", .file)])))
        tree.renamed("old", to: "new")
        XCTAssertEqual(tree.rows.map(\.path), ["new", "new/f"])
    }

    func testRenameSelectsTheStemNotTheExtension() {
        XCTAssertEqual(FileNames.stemLength("main.rs", isFolder: false), 4)
        XCTAssertEqual(FileNames.stemLength("archive.tar.gz", isFolder: false), 11)
        XCTAssertEqual(FileNames.stemLength(".gitignore", isFolder: false), 10)
        XCTAssertEqual(FileNames.stemLength("trailing.", isFolder: false), 9)
        XCTAssertEqual(FileNames.stemLength("v1.2", isFolder: true), 4)
    }

    func testPathMenuListsParentsPinsAndSaysWhereTwinsAre() {
        let menu = PathMenuModel(shown: "/Users/ben/code/app", volume: "Macintosh HD", pins: ["/Users/ben/a/app", "/Users/ben/b/app", "/srv/www"], chosen: true, home: "/Users/ben")
        XCTAssertEqual(menu.folders.map(\.name), ["app", "code", "ben", "Users", "Macintosh HD"])
        XCTAssertEqual(menu.pinned.map(\.name), ["app \u{2014} ~/a", "app \u{2014} ~/b", "www"])
        XCTAssertTrue(menu.canFollow)
        XCTAssertFalse(menu.isPinned)
        XCTAssertFalse(PathMenuModel.onBootVolume("/Volumes/Work/a"))
        XCTAssertTrue(PathMenuModel.onBootVolume("/Volumesx/a"))
    }

    func testIconThemeLooksUpLongestSuffixThenDefault() throws {
        let theme = try XCTUnwrap(BundledIconTheme())
        let rust = try XCTUnwrap(theme.file("src/main.rs", dark: true))
        XCTAssertTrue(rust.lastPathComponent.contains("rust"), "\(rust)")
        XCTAssertEqual(theme.file("PHOTO.PNG", dark: false)?.lastPathComponent, theme.file("a.png", dark: false)?.lastPathComponent)
        XCTAssertNotNil(theme.file("no-such-kind.zzz", dark: true), "the default icon")
        XCTAssertNotEqual(theme.file("main.rs", dark: true), theme.file("main.rs", dark: false), "a variant per appearance")
        XCTAssertNotNil(theme.folder("src", open: true, dark: true))
        XCTAssertEqual(BundledIconTheme.candidates("foo.d.ts"), ["foo.d.ts", "d.ts", "ts"])
        XCTAssertEqual(BundledIconTheme.candidates(".gitignore"), [".gitignore", "gitignore"])
    }
}

import AppKit
@testable import StudioModel
@testable import StudioUI
import XCTest

/// Export JSON and Copy JSON through their seams: a scratch folder and a captured string, never a system panel, the
/// person's files or their pasteboard.
@MainActor
final class OutputsTests: XCTestCase {
    private func session(_ fixtures: StudioFixtures) throws -> StudioSession {
        try StudioSession(theme: "Nord", environment: fixtures.environment, store: fixtures.store)
    }

    func testExportWritesTheThemeWhereTheSeamSays() async throws {
        let fixtures = try StudioFixtures()
        defer { fixtures.remove() }
        let studio = try session(fixtures)
        let out = fixtures.folder.directory.appendingPathComponent("out")
        try FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)
        var asked: String?
        studio.outputs.exportDestination = { name, _, _ in asked = name; return out.appendingPathComponent(name) }

        await studio.exportJSON()

        let name = try XCTUnwrap(asked)
        XCTAssertEqual(name, studio.document.exportFileName)
        XCTAssertEqual(try String(contentsOf: out.appendingPathComponent(name), encoding: .utf8), studio.document.exportJSON)
        XCTAssertEqual(studio.document.notice, .exported(out.appendingPathComponent(name), note: nil))
    }

    func testACancelledPanelWritesNothing() async throws {
        let fixtures = try StudioFixtures()
        defer { fixtures.remove() }
        let studio = try session(fixtures)
        studio.outputs.exportDestination = { _, _, _ in nil }

        await studio.exportJSON()

        XCTAssertNil(studio.document.notice)
    }

    func testCopyJSONGoesToTheSeamAndLeavesThePasteboardAlone() throws {
        let fixtures = try StudioFixtures()
        defer { fixtures.remove() }
        let studio = try session(fixtures)
        var copied: String?
        studio.outputs.copy = { copied = $0 }
        let before = NSPasteboard.general.changeCount

        studio.copyJSON()

        XCTAssertEqual(copied, studio.document.exportJSON)
        XCTAssertEqual(studio.document.notice, .copiedJSON(note: nil))
        XCTAssertEqual(NSPasteboard.general.changeCount, before)
    }

    func testEverySpecimenHasItsOwnContentAndAnArea() {
        let ids = GalleryCatalog.specimens.map(\.id)
        XCTAssertEqual(Set(ids).count, ids.count)
        // The GPUI gallery's topics (`studio/gallery/`), each as a specimen of its own.
        for topic in ["sidebar", "tiles", "changes", "diff", "chrome", "find", "panes", "chips", "labels", "shape", "controls", "buttons", "inputs", "menus", "windows", "palette", "terminal", "code", "type", "metrics"] {
            XCTAssertTrue(ids.contains(topic), topic)
        }
    }
}

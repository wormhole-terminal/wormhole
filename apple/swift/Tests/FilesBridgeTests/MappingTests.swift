import PanelModel
import WormholeBindings
import WorkspaceModel
import XCTest

@testable import FilesBridge

/// The records Rust sends, canned, as the values the panel draws: no Rust runs here.
final class MappingTests: XCTestCase {
    func testADirectoryKeepsTheRealNameAndTheDrawnLabel() {
        let record = DirectoryRecord(
            entries: [
                EntryRecord(name: "evil\nname", label: "evil\u{FFFD}name", kind: .file),
                EntryRecord(name: "src", label: "src", kind: .folder),
                EntryRecord(name: "jump", label: "jump", kind: .link),
            ], more: 3, atLeast: true)
        let directory = Directory(record)
        XCTAssertEqual(directory.entries.map(\.name), ["evil\nname", "src", "jump"])
        XCTAssertEqual(directory.entries[0].label, "evil\u{FFFD}name")
        XCTAssertEqual(directory.entries.map(\.kind), [.file, .folder, .link])
        XCTAssertEqual(directory.entries[2].isFolder, false, "a link never opens")
        XCTAssertEqual(directory.more, 3)
        XCTAssertTrue(directory.atLeast)
    }

    func testFileContentAndFailuresMapOneToOne() {
        XCTAssertEqual(FileContent(.text(text: "a", size: 9, truncated: true)), .text("a", size: 9, truncated: true))
        XCTAssertEqual(FileContent(.binary(size: 5)), .binary(size: 5))
        XCTAssertEqual(FileContent(.image(bytes: Data([1, 2]), size: 2)), .image(Data([1, 2]), size: 2))
        XCTAssertEqual(FileContent(.unavailable(problem: .outside)), .unavailable(.outside))
        XCTAssertEqual(DirectoryFailure(FilesFailure.Denied), .denied)
        XCTAssertEqual(PanelModel.RenameFailure(WormholeBindings.RenameFailure.Exists), .exists)
        XCTAssertEqual(PanelModel.RenameFailure(CocoaError(.fileNoSuchFile)), .other, "anything unknown is 'other'")
    }

    func testAProjectStateCarriesRustsWordingAndDropsASwitchItDoesNotKnow() {
        let record = ProjectStateRecord(
            root: "/work/app", status: .changed, commands: [TrustLineRecord(what: "action t:", runs: "make", note: "(current)")],
            applies: "its settings in config.json", asksForTrust: true,
            features: [
                ProjectFeatureRecord(key: "actions", on: true, hint: "Needs trust before it takes effect", warns: true),
                ProjectFeatureRecord(key: "from-the-future", on: true, hint: "?", warns: false),
            ],
            actions: [ProjectActionRecord(id: "t", title: "Tests", detail: "make", key: "\u{2303}\u{2318}T")],
            prompts: [ProjectPromptRecord(id: "r", title: "Review", detail: nil)], layout: ["dev"], errors: ["boom"], warnings: ["w"])
        let state = ProjectState(record)
        XCTAssertEqual(state.status, .changed)
        XCTAssertEqual(state.commands, [TrustLine(what: "action t:", runs: "make", note: "(current)")])
        XCTAssertEqual(state.features, [FeatureRow(.actions, on: true, hint: "Needs trust before it takes effect", warns: true)])
        XCTAssertEqual(state.actions[0].key, "\u{2303}\u{2318}T")
        XCTAssertEqual(state.notice.title, "Files changed since you trusted this project")
        XCTAssertEqual([state.layout, state.errors, state.warnings], [["dev"], ["boom"], ["w"]])
    }

    func testAFolderPaintNamesAColourOrAHex() {
        let named = FolderPaint(FolderPaintRecord(chosen: "teal", custom: nil, locked: false, note: "n"))
        XCTAssertEqual(named.chosen, .teal)
        let custom = FolderPaint(FolderPaintRecord(chosen: nil, custom: "#7fb2ff", locked: true, note: "n"))
        XCTAssertEqual(custom.custom, "#7fb2ff")
        XCTAssertFalse(custom.enabled)
    }

    func testEffectsMapAndABadURLIsDropped() {
        XCTAssertEqual(
            PanelEffect(.command(place: .named(name: "dev", cwd: "/w"), command: "make")),
            .type("make", in: .named("dev", cwd: "/w")))
        XCTAssertEqual(PanelEffect(.paste(place: .current, text: "t", submit: false)), .paste("t", submit: false, in: .current))
        XCTAssertEqual(PanelEffect(.openUrl(url: "https://example.com")), .open(URL(string: "https://example.com")!))
        XCTAssertNil(PanelEffect(.openUrl(url: "not a url")))
        let layout = PanelEffect(.openTerminals(terminals: [NewTerminalRecord(name: "a", cwd: "/w", run: nil, focus: true)]))
        XCTAssertEqual(layout, .openTerminals([NewTerminal(name: "a", cwd: "/w", run: nil, focus: true)]))
    }

    func testUsageRecordsBecomeReports() {
        let report = UsageReport(
            UsageReportRecord(
                agent: "claude", limits: [PlanLimitRecord(label: "5-hour limit", used: 25, resetsAt: 1_800_002_040)],
                responseTokens: nil, updated: 1_800_000_000))
        XCTAssertEqual(report.limits, [PlanLimit(label: "5-hour limit", used: 25, resetsAt: 1_800_002_040)])
        XCTAssertEqual(report.limits[0].shown(now: 1_800_000_000).words, "25% \u{b7} resets 34m")
        XCTAssertEqual(report.displayName, "Claude")
    }
}

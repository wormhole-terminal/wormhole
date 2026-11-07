import PanelModel
import XCTest

@testable import PanelUI

@MainActor
final class StoreTests: XCTestCase {
    private func settle() async throws { try await Task.sleep(for: .milliseconds(200)) }

    private func controller(host: MockPanelHost = MockPanelHost(), made: MadeWorktree? = nil) async throws -> PanelController {
        let c = PanelController(host: host, backends: .mock(made: made), notes: PanelSamples.notes)
        c.follow(folder: "/p")
        try await settle()
        return c
    }

    func testChangesLoadOpenStepAndCompare() async throws {
        let c = try await controller()
        XCTAssertEqual(c.changes.files.count, 6)
        XCTAssertEqual(c.badge(for: .changes), 6)
        let first = c.changes.order[0]
        c.changes.open(first.id)
        try await settle()
        XCTAssertNotNil(c.changes.diff.parsed)
        c.changes.step(1)
        XCTAssertNotEqual(c.changes.selection, first.id)
        c.changes.setCompare(Rev(name: "main"))
        XCTAssertNil(c.changes.selection, "a new comparison closes the diff")
        XCTAssertEqual(c.changes.compare?.title, "main")
        XCTAssertFalse(c.changes.canReview, "comments are only for the working tree")
        c.changes.setCompare(nil)
        XCTAssertTrue(c.changes.canReview)
    }

    func testPastingAReviewClearsItOnlyWhereTheTerminalTakesAPaste() async throws {
        let host = MockPanelHost()
        let c = try await controller(host: host)
        c.changes.review.put(ReviewComment(path: "a.rs", side: .new, line: 3, code: "x", text: "fix"))
        host.acceptsPaste = false
        c.changes.pasteReview()
        XCTAssertTrue(c.changes.copiedInstead)
        XCTAssertEqual(host.copied.count, 1)
        XCTAssertFalse(c.changes.review.isEmpty, "the comments stay")
        host.acceptsPaste = true
        c.changes.pasteReview()
        XCTAssertEqual(host.pasted.count, 1)
        XCTAssertTrue(c.changes.review.isEmpty)
    }

    func testHistoryOpenCommitFileDiffAndFilter() async throws {
        let c = try await controller()
        let sha = c.history.model.commits[0].id
        c.history.toggle(sha)
        try await settle()
        XCTAssertEqual(c.history.files.count, 2)
        c.history.open(c.history.files[0].id)
        try await settle()
        XCTAssertNotNil(c.history.diff.parsed)
        c.history.toggle(sha)
        XCTAssertNil(c.history.selection, "closing the commit closes its diff")
        c.showFileHistory(PanelSamples.files[0].path)
        try await settle()
        XCTAssertEqual(c.tab, .history)
        XCTAssertEqual(c.history.model.commits.count, 1)
    }

    func testFilesClickRenamePinAndViewer() async throws {
        let host = MockPanelHost()
        let c = try await controller(host: host)
        let f = c.files
        f.click(f.rows.first { $0.path == "docs" }!)
        try await settle()
        XCTAssertTrue(f.rows.contains { $0.path == "docs/ARCHITECTURE.md" })
        f.click(f.rows.first { $0.path == "README.md" }!)
        try await settle()
        XCTAssertEqual(f.viewing, "README.md")
        XCTAssertEqual(f.lines.first, "# Wormhole")
        f.startRename(f.rows.first { $0.path == "README.md" }!)
        f.commitRename(to: "Cargo.toml")
        try await settle()
        XCTAssertNotNil(f.renameError, "an existing name is refused and the field stays")
        f.commitRename(to: "READ.md")
        try await settle()
        XCTAssertNil(f.renaming)
        XCTAssertEqual(f.viewing, "READ.md", "the viewer follows the new name")
        XCTAssertTrue(host.notices.contains("Renamed to READ.md"))
        f.setPinned("docs", true)
        try await settle()
        XCTAssertTrue(f.isPinned("docs"))
        f.step(1)
        f.fold(open: false)
        f.chooseFolder()
        host.folder = "/elsewhere"
        f.choose("/elsewhere")
        XCTAssertEqual(f.root, "/elsewhere")
        f.followTerminal()
        XCTAssertEqual(f.root, "/p")
    }

    func testNotesSendStashAndMove() async throws {
        let host = MockPanelHost()
        let c = try await controller(host: host)
        let notes = c.notes
        notes.send(notes.model.tasks.items[1])
        XCTAssertEqual(host.sent, ["Memoise the pinned-first order"])
        XCTAssertFalse(notes.stashPrompt(), "nothing at the prompt")
        host.prompt = "half a thought"
        XCTAssertTrue(notes.stashPrompt())
        XCTAssertEqual(notes.model.notes.last?.text, "half a thought")
        XCTAssertNil(host.prompt)
        let before = notes.model.tasks.items.map(\.text)
        notes.moveTask(from: 0, to: 2)
        XCTAssertEqual(notes.model.tasks.items.map(\.text), [before[1], before[2], before[0], before[3]])
    }

    func testProjectTrustFeaturesAndColour() async throws {
        let c = try await controller()
        XCTAssertEqual(c.project.state?.status, .untrusted)
        c.project.approve()
        try await settle()
        XCTAssertEqual(c.project.state?.status, .trusted)
        c.project.setFeature(.layout, on: true)
        c.project.setColor(.teal)
        try await settle()
        XCTAssertEqual(c.project.state?.features.first { $0.feature == .layout }?.on, true)
        XCTAssertEqual(c.project.paint.chosen, .teal)
        c.project.copyToNote(PanelSamples.project.prompts[0], in: c.notes)
        try await settle()
        XCTAssertEqual(c.notes.model.notes.count, 3)
    }

    func testWorktreeAskCreateAndRemove() async throws {
        let made = MadeWorktree(repo: "/p", branch: "fix/x", folder: "/p-worktrees/fix-x")
        let c = try await controller(made: made)
        let w = c.worktrees
        XCTAssertEqual(w.made, made)
        w.ask()
        try await settle()
        XCTAssertTrue(w.asking)
        w.branch = "bad name"
        try await settle()
        XCTAssertFalse(w.canCreate)
        w.branch = "feature/new"
        try await settle()
        XCTAssertTrue(w.canCreate)
        XCTAssertTrue(w.footnote?.hasPrefix("Folder: ") == true)
        w.create()
        try await settle()
        XCTAssertFalse(w.asking)
        w.dirty = true
        w.askRemoval()
        XCTAssertFalse(w.confirmingRemoval, "a worktree with changes stays")
        w.dirty = false
        w.askRemoval()
        XCTAssertTrue(w.confirmingRemoval)
        w.confirmRemoval()
        try await settle()
        XCTAssertNil(w.made)
    }

    func testUsageBadgeAndCoverFollowPresentation() async throws {
        let c = try await controller()
        XCTAssertEqual(c.badge(for: .usage), 3)
        XCTAssertNil(c.badge(for: .history))
        c.tab = .changes
        c.changes.open(c.changes.files[0].id)
        XCTAssertNil(c.cover, "in the panel there is nothing for the window to host")
        c.presentation = .terminalArea
        XCTAssertEqual(c.cover, .changesDiff)
    }
}

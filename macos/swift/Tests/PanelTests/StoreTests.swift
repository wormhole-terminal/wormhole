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

    /// The comment being written is the store's, above the diff's rows (`diff/review.rs`'s `Commenting`): a reload, even one
    /// that closes the diff or takes its line away, leaves it and its text; it is saved into the review of the terminal it
    /// was opened for, whichever the tab shows by then.
    func testACommentBeingWrittenOutlivesAReloadAndGoesToItsTerminal() async throws {
        let c = try await controller()
        let changes = c.changes
        changes.follow("/p", terminal: 1)
        try await settle()
        changes.open(changes.order[0].id)
        try await settle()
        changes.commenting = ReviewComment(path: "gone.rs", side: .new, line: 3, code: "let x = 1;", text: "")
        changes.commenting?.text = "name it"
        // The file changed under it: the list and the diff are read again, and the diff closes.
        changes.refresh()
        changes.close()
        try await settle()
        XCTAssertEqual(changes.commenting?.text, "name it", "nothing a reload does reaches what is being written")
        XCTAssertFalse(changes.commentingExisting)
        // Another terminal comes to the tab before it is saved: the comment is still the first one's.
        changes.follow("/p", terminal: 2)
        XCTAssertEqual(changes.commenting?.text, "name it")
        changes.saveComment()
        XCTAssertNil(changes.commenting)
        XCTAssertTrue(changes.review.isEmpty, "not the terminal shown now")
        changes.follow("/p", terminal: 1)
        XCTAssertEqual(changes.review.on("gone.rs", .new, 3)?.text, "name it", "on the line it was opened on")
        // Opened again on that line, it is an old comment: Remove takes it away.
        changes.commenting = ReviewComment(path: "gone.rs", side: .new, line: 3, code: "let x = 1;", text: "name it")
        XCTAssertTrue(changes.commentingExisting)
        changes.removeComment()
        XCTAssertNil(changes.commenting)
        XCTAssertTrue(changes.review.isEmpty)
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
        c.showFileHistory([PanelSamples.files[0].path])
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
        // The keys' press: a folder toggles, a file shows in the viewer (the walk itself is the core's, tested over the FFI).
        f.press(WalkRow(id: "docs", level: 0, isOpen: true), space: false, following: false)
        XCTAssertFalse(f.rows.contains { $0.path == "docs/ARCHITECTURE.md" })
        f.chooseFolder()
        host.folder = "/elsewhere"
        f.choose("/elsewhere")
        XCTAssertEqual(f.root, "/elsewhere")
        f.followTerminal()
        XCTAssertEqual(f.root, "/p")
    }

    func testMarksAndTheKeysOnTheChangesList() async throws {
        let c = try await controller()
        let list = c.changes
        let files = list.order
        XCTAssertGreaterThan(files.count, 2)
        list.click(files[0], command: true, shift: false)
        list.click(files[2], command: true, shift: false)
        XCTAssertEqual(list.marked, [files[0].path, files[2].path])
        XCTAssertNil(list.selection, "a mark opens no diff")
        XCTAssertEqual(list.menuPaths(for: files[2]), [files[0].path, files[2].path], "a marked row's menu acts on every marked file")
        XCTAssertEqual(list.menuPaths(for: files[1]), [files[1].path], "an unmarked row's menu is its own")
        list.click(files[1], command: false, shift: true)
        XCTAssertEqual(list.marked.count, 3, "shift marks the range from the last clicked file")
        list.click(files[1], command: false, shift: false)
        XCTAssertTrue(list.marked.isEmpty, "a plain click opens the diff and drops the marks")
        XCTAssertEqual(list.selection, files[1].id)
        // A band folds only on Space; a file's press opens its diff, and the diff then follows the cursor.
        let rows = list.walkRows
        XCTAssertTrue(rows.contains { $0.isBand })
        XCTAssertTrue(list.following)
        list.press(rows.first { $0.name == files[0].name && !$0.isBand }!, space: false, following: true)
        XCTAssertEqual(list.selection, files[0].id)
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
        let found = await w.made(at: "/p-worktrees/fix-x")
        XCTAssertEqual(found, made)
        w.ask()
        try await settle()
        XCTAssertTrue(w.asking)
        XCTAssertEqual(w.footnote, "Folder: <branch>")
        w.branch = "bad name"
        try await settle()
        XCTAssertTrue(w.canCreate, "Create answers a refused branch with its reason instead of going grey")
        w.create()
        try await settle()
        XCTAssertTrue(w.asking, "a refused branch asks nothing of git")
        w.branch = "feature/new"
        try await settle()
        XCTAssertTrue(w.footnote?.hasPrefix("Folder: ") == true)
        w.create()
        try await settle()
        XCTAssertFalse(w.asking)
        await w.remove(made)
        XCTAssertNil(w.alert)
    }

    func testUsageBadgeAndCoverFollowPresentation() async throws {
        let c = try await controller()
        XCTAssertNil(c.badge(for: .usage), "only Changes has a count on its tab")
        XCTAssertNil(c.badge(for: .history))
        c.tab = .changes
        c.changes.open(c.changes.files[0].id)
        XCTAssertNil(c.cover, "in the panel there is nothing for the window to host")
        c.presentation = .terminalArea
        XCTAssertEqual(c.cover, .changesDiff)
    }
}

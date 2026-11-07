import DesignSystem
import FileIconTheme
import PanelModel
import SwiftUI
import XCTest

@testable import PanelUI

/// Renders every tab, opened and closed, in several themes: the files to look at are under `PANEL_SNAPSHOTS`.
@MainActor
final class ViewSnapshotTests: XCTestCase {
    /// Themes whose light/dark and accent differ, and the two default ones under Increase Contrast.
    static let looks: [(id: String, contrast: Bool)] = [
        ("wormhole.dark", false), ("wormhole.light", false), ("catppuccin_mocha", false), ("github_light", false),
        ("nord", false), ("wormhole.dark", true), ("wormhole.light", true),
    ]

    static let folder = "/Users/benjamin/Projects/wormhole"

    func each<V: View>(_ name: String, _ view: @autoclosure () -> V, size: CGSize? = nil, looks: [(id: String, contrast: Bool)]? = nil) throws {
        for look in looks ?? Self.looks {
            let url = try Snapshot.write(
                view().environment(\.fileIcons, BundledIconTheme()), name: name, theme: look.id, contrast: look.contrast,
                size: size ?? CGSize(width: 360, height: 620))
            XCTAssertGreaterThan((try? Data(contentsOf: url).count) ?? 0, 2_000, name)
        }
    }

    /// A controller that has read its folder, as the window's has by the time a tab is looked at.
    func controller(_ tab: PanelTab, made: MadeWorktree? = nil) async throws -> PanelController {
        let controller = PanelController(host: MockPanelHost(), backends: .mock(made: made), notes: PanelSamples.notes, tab: tab)
        controller.now = PanelSamples.now
        controller.follow(folder: Self.folder)
        try await Task.sleep(for: .milliseconds(250))
        return controller
    }

    func testNotes() async throws {
        let notes = try await controller(.notes)
        try each("notes", PanelContentView(controller: notes))
        try each("notes-empty", NotesView(store: NotesStore()))
    }

    func testChanges() async throws {
        let c = try await controller(.changes)
        let files = c.changes.files
        try each("changes", PanelContentView(controller: c))
        c.changes.layout.tree = false
        try each("changes-flat", PanelContentView(controller: c))
        c.changes.layout.tree = true
        c.changes.open(files[0].id)
        try await Task.sleep(for: .milliseconds(250))
        try each("changes-diff", PanelContentView(controller: c), size: CGSize(width: 360, height: 760))
        c.changes.open(files[2].id)
        try await Task.sleep(for: .milliseconds(250))
        try each("changes-diff-split", PanelContentView(controller: c), size: CGSize(width: 900, height: 620))
        c.changes.open(files[4].id)
        try await Task.sleep(for: .milliseconds(250))
        try each("changes-binary", PanelContentView(controller: c))
    }

    func testChangesReview() async throws {
        let c = try await controller(.changes)
        c.changes.open(c.changes.files[0].id)
        try await Task.sleep(for: .milliseconds(250))
        c.changes.review.put(ReviewComment(path: c.changes.files[0].path, side: .new, line: 44, code: "self.rows.iter().filter(|row| row.visible())", text: "Cache this."))
        try each("changes-review", PanelContentView(controller: c), size: CGSize(width: 360, height: 760), looks: Array(Self.looks.prefix(2)))
        try each("changes-review-wide", PanelContentView(controller: c), size: CGSize(width: 900, height: 620), looks: Array(Self.looks.prefix(2)))
    }

    func testCoverPresentation() async throws {
        let c = try await controller(.changes)
        c.presentation = .terminalArea
        c.changes.open(c.changes.files[0].id)
        try await Task.sleep(for: .milliseconds(250))
        XCTAssertEqual(c.cover, .changesDiff)
        try each("cover-diff", PanelCoverView(controller: c), size: CGSize(width: 800, height: 520), looks: Array(Self.looks.prefix(2)))
        try each("cover-list", PanelContentView(controller: c), looks: Array(Self.looks.prefix(2)))
    }

    func testHistory() async throws {
        let c = try await controller(.history)
        try each("history", PanelContentView(controller: c))
        c.history.toggle(c.history.model.commits[0].id)
        try await Task.sleep(for: .milliseconds(250))
        try each("history-open", PanelContentView(controller: c), size: CGSize(width: 360, height: 760))
        c.history.open(c.history.files[0].id)
        try await Task.sleep(for: .milliseconds(250))
        try each("history-diff", PanelContentView(controller: c), size: CGSize(width: 360, height: 800))
        try each("history-empty", HistoryView(store: HistoryStore(git: MockGitBackend(history: HistoryModel()), highlighter: LexerHighlighter())))
    }

    func testFiles() async throws {
        let c = try await controller(.files)
        c.files.toggle("crates")
        try await Task.sleep(for: .milliseconds(150))
        c.files.toggle("crates/wormhole")
        try await Task.sleep(for: .milliseconds(150))
        c.files.toggle("crates/wormhole/src")
        try await Task.sleep(for: .milliseconds(150))
        try each("files", PanelContentView(controller: c), size: CGSize(width: 360, height: 700))
        c.files.view("crates/wormhole/src/main.rs")
        try await Task.sleep(for: .milliseconds(350))
        try each("files-viewer", PanelContentView(controller: c), size: CGSize(width: 360, height: 820))
        c.files.closeViewer()
        c.files.startRename(c.files.rows.first { $0.name == "Cargo.toml" }!)
        try each("files-rename", PanelContentView(controller: c), size: CGSize(width: 360, height: 700), looks: Array(Self.looks.prefix(2)))
        c.files.cancelRename()
        c.files.view("icon.png")
        try await Task.sleep(for: .milliseconds(200))
        try each("files-binary", PanelContentView(controller: c), looks: Array(Self.looks.prefix(2)))
    }

    func testProject() async throws {
        let c = try await controller(.project)
        try each("project", PanelContentView(controller: c), size: CGSize(width: 360, height: 1100))
        c.project.approve()
        try await Task.sleep(for: .milliseconds(250))
        try each("project-trusted", PanelContentView(controller: c), size: CGSize(width: 360, height: 900), looks: Array(Self.looks.prefix(3)))
        let bare = PanelController(host: MockPanelHost(), backends: PanelBackends(
            git: MockGitBackend(), files: MockFilesBackend(), project: MockProjectBackend(state: nil), usage: MockUsageBackend(), worktrees: MockWorktreeBackend()), tab: .project)
        bare.follow(folder: Self.folder)
        try await Task.sleep(for: .milliseconds(250))
        try each("project-none", PanelContentView(controller: bare), looks: Array(Self.looks.prefix(3)))
    }

    func testUsage() async throws {
        let c = try await controller(.usage)
        try each("usage", UsageView(store: c.usage, now: PanelSamples.now))
        try each("usage-empty", UsageView(store: UsageStore(backend: MockUsageBackend([]))))
    }

    func testWorktreeSheet() async throws {
        let c = try await controller(.changes, made: MadeWorktree(repo: Self.folder, branch: "fix/login", folder: "/Users/benjamin/Projects/wormhole-worktrees/fix-login"))
        c.worktrees.branch = "fix/login"
        try await Task.sleep(for: .milliseconds(150))
        try each("worktree-sheet", NewWorktreeSheet(store: c.worktrees), size: CGSize(width: 420, height: 280), looks: Array(Self.looks.prefix(2)))
    }
}

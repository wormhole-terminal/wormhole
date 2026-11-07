// Mock data for previews, screenshots and tests. The real source arrives later through the Rust core.

import Foundation

public enum PanelSamples {
    public static let now = Date(timeIntervalSince1970: 1_790_000_000)

    public static let notes = NotesModel(
        text: """
            Sidebar flicker comes from the list rebuilding on every tick.
            Check whether the pinned rows are re-sorted each frame before touching the cache.
            """,
        tasks: TaskList([
            PanelTask(text: "Reproduce the flicker with 40 terminals", done: true),
            PanelTask(text: "Memoise the pinned-first order"),
            PanelTask(text: "Add a counting test for rebuilt rows"),
            PanelTask(text: "Write the changelog line"),
        ]))

    private static let files: [ChangedFile] = [
        ChangedFile(path: "crates/wormhole/src/sidebar/list.rs", status: .modified, staged: true, added: 9, removed: 4),
        ChangedFile(path: "apple/swift/Sources/PanelModel/TaskList.swift", status: .added, staged: true, added: 5),
        ChangedFile(path: "crates/wormhole-core/src/config/settings.rs", status: .modified, staged: false, added: 1, removed: 1),
        ChangedFile(path: "docs/notes-panel.md", status: .renamed, staged: false, added: 1, removed: 1),
        ChangedFile(path: "packaging/icon.png", status: .modified, staged: false),
        ChangedFile(path: "scratch.txt", status: .untracked, staged: false),
    ]

    public static let changes = ChangesModel(
        files: files,
        diffs: [
            files[0].id: SampleDiffs.sidebar, files[1].id: SampleDiffs.newFile,
            files[2].id: SampleDiffs.settings, files[3].id: SampleDiffs.renamed,
            files[4].id: SampleDiffs.binary,
        ])

    public static let history = HistoryModel(commits: [
        commit("9f3c1ab2e", "fix(sidebar): keep pinned rows in place under a filter", "Benny", minutes: 12, files: [files[0], files[2]]),
        commit("c07d5e41a", "feat(panel): next steps with a ticking animation", "Benny", minutes: 190, files: [files[1], files[3]]),
        commit("5b1e9d03f", "perf(term): parse rows without copying the styles", "Mara Lindqvist", minutes: 60 * 27, files: [files[2]]),
        commit("e2a44b7c8", "docs: how persistent terminals reattach", "Mara Lindqvist", minutes: 60 * 24 * 4, files: [files[3]]),
        commit("0d8f6a315", "chore: wormhole 1.6.3", "Benny", minutes: 60 * 24 * 40),
    ])

    private static func commit(_ hash: String, _ subject: String, _ author: String, minutes: Int, files: [ChangedFile] = []) -> Commit {
        Commit(hash: hash, subject: subject, author: author, date: now.addingTimeInterval(Double(-minutes * 60)), files: files)
    }
}

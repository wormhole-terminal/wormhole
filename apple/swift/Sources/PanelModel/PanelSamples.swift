// Mock data for previews, screenshots and tests. The real source arrives later through the Rust core.

import Foundation

public enum PanelSamples {
    public static let now = 1_790_000_000

    public static let notes = NotesModel(
        notes: [
            Note(text: "Sidebar flicker\nComes from the list rebuilding on every tick.\nCheck whether the pinned rows are re-sorted each frame before touching the cache."),
            Note(text: "Release checklist"),
        ],
        tasks: TaskList([
            PanelTask(text: "Reproduce the flicker with 40 terminals", done: true),
            PanelTask(text: "Memoise the pinned-first order"),
            PanelTask(text: "Add a counting test for rebuilt rows"),
            PanelTask(text: "Write the changelog line"),
        ]))

    private static let files: [ChangedFile] = [
        ChangedFile(path: "crates/wormhole/src/sidebar/list.rs", kind: .modified, added: 9, removed: 4),
        ChangedFile(path: "apple/swift/Sources/PanelModel/TaskList.swift", kind: .added, added: 5),
        ChangedFile(path: "crates/wormhole-core/src/config/settings.rs", kind: .modified, added: 1, removed: 1),
        ChangedFile(path: "docs/notes-panel.md", kind: .renamed, added: 1, removed: 1, originalPath: "docs/panel.md"),
        ChangedFile(path: "packaging/icon.png", kind: .modified, added: nil, removed: nil),
        ChangedFile(path: "scratch.txt", kind: .untracked, added: 8),
    ]

    public static let changes = ChangesModel(
        files: files,
        diffs: [
            files[0].id: SampleDiffs.sidebar, files[1].id: SampleDiffs.newFile,
            files[2].id: SampleDiffs.settings, files[3].id: SampleDiffs.renamed,
            files[4].id: SampleDiffs.binary,
        ])

    private static let me = "Benny"

    public static let history = HistoryModel(
        commits: [
            commit("9f3c1ab2e4d07", "fix(sidebar): keep pinned rows in place under a filter", me, minutes: 12, files: [files[0], files[2]], refs: [(.head, "HEAD"), (.branch, "main")], body: "A filter used to re-sort the pinned rows, so a pin jumped while typing.\n\nThe order is now kept per list and the filter only hides rows."),
            commit("c07d5e41a9b22", "feat(panel): next steps with a ticking animation", me, minutes: 190, files: [files[1], files[3]]),
            commit("5b1e9d03f7a11", "perf(term): parse rows without copying the styles", "Mara Lindqvist", minutes: 60 * 27, files: [files[2]]),
            commit("e2a44b7c8d3f0", "docs: how persistent terminals reattach", "Mara Lindqvist", minutes: 60 * 24 * 4, files: [files[3]]),
            commit("0d8f6a3155be9", "chore: wormhole 1.6.3", me, minutes: 60 * 24 * 40),
        ], branch: "main")

    private static func commit(
        _ sha: String, _ subject: String, _ author: String, minutes: Int, files: [ChangedFile] = [],
        refs: [(kind: RefKind, name: String)] = [], body: String = ""
    ) -> Commit {
        Commit(
            sha: sha, subject: subject, body: body,
            author: Signature(name: author, email: "\(author.split(separator: " ")[0].lowercased())@example.com", time: now - minutes * 60, zone: 7200),
            parents: ["a1b2c3d4e5f60", "b2c3d4e5f6a71"].prefix(sha.hasPrefix("5") ? 2 : 1).map { $0 }, refs: refs, files: files)
    }
}

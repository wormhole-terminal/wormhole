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

    public static let files: [ChangedFile] = [
        ChangedFile(path: "crates/wormhole/src/sidebar/list.rs", kind: .modified, added: 9, removed: 4),
        ChangedFile(path: "apple/swift/Sources/PanelModel/TaskList.swift", kind: .added, added: 5),
        ChangedFile(path: "crates/wormhole-core/src/config/settings.rs", kind: .modified, added: 1, removed: 1),
        ChangedFile(path: "docs/notes-panel.md", kind: .renamed, added: 1, removed: 1, originalPath: "docs/panel.md"),
        ChangedFile(path: "packaging/icon.png", kind: .modified, added: nil, removed: nil),
        ChangedFile(path: "scratch.txt", kind: .untracked, added: 8),
    ]

    public static let changes = ChangesModel(files: files)

    /// The parsed diff of each sample file.
    public static let diffs: [ChangedFile.ID: DiffFile] = [
        files[0].id: SampleDiffs.sidebar, files[1].id: SampleDiffs.newFile, files[2].id: SampleDiffs.settings,
        files[3].id: SampleDiffs.renamed, files[4].id: SampleDiffs.binary,
    ]

    private static let me = "Benny"

    /// A sample commit: its log row, the files it changed and the body of its message.
    private struct Sample {
        var commit: Commit
        var files: [ChangedFile]
        var body: String
    }

    private static func sample(
        _ sha: String, _ subject: String, _ author: String, minutes: Int, files: [ChangedFile] = [],
        refs: [(kind: RefKind, name: String)] = [], body: String = ""
    ) -> Sample {
        let parents = ["a1b2c3d4e5f60", "b2c3d4e5f6a71"]
        let commit = Commit(
            sha: sha, subject: subject, author: author, time: now - minutes * 60,
            parents: [parents[0]], refs: refs, merge: sha.hasPrefix("5"))
        return Sample(commit: commit, files: files, body: body)
    }

    private static let samples = [
        sample("9f3c1ab2e4d07", "fix(sidebar): keep pinned rows in place under a filter", me, minutes: 12, files: [files[0], files[2]], refs: [(.head, "HEAD"), (.branch, "main")], body: "A filter used to re-sort the pinned rows, so a pin jumped while typing.\n\nThe order is now kept per list and the filter only hides rows."),
        sample("c07d5e41a9b22", "feat(panel): next steps with a ticking animation", me, minutes: 190, files: [files[1], files[3]]),
        sample("5b1e9d03f7a11", "perf(term): parse rows without copying the styles", "Mara Lindqvist", minutes: 60 * 27, files: [files[2]]),
        sample("e2a44b7c8d3f0", "docs: how persistent terminals reattach", "Mara Lindqvist", minutes: 60 * 24 * 4, files: [files[3]]),
        sample("0d8f6a3155be9", "chore: wormhole 1.6.3", me, minutes: 60 * 24 * 40),
    ]

    public static let history = HistoryModel(commits: samples.map(\.commit), branch: "main")

    /// The commits that touched any of `paths`, for a log limited to some files.
    static func commits(touching paths: [String]) -> [Commit] {
        samples.filter { $0.files.contains { paths.contains($0.path) } }.map(\.commit)
    }

    /// The opened commit's block, written plainly (the core writes the real one in the user's language).
    static func details(of sha: String) -> CommitDetails? {
        guard let found = samples.first(where: { $0.commit.sha == sha }) else { return nil }
        let (commit, files) = (found.commit, found.files)
        let email = "\(commit.author.split(separator: " ")[0].lowercased())@example.com"
        let (added, removed) = (files.compactMap(\.added).reduce(0, +), files.compactMap(\.removed).reduce(0, +))
        typealias Fact = CommitDetails.Fact
        let parents = ["a1b2c3d4e5f60", "b2c3d4e5f6a71"].prefix(commit.merge ? 2 : 1).map {
            CommitDetails.Parent(sha: $0, short: String($0.prefix(7)), spoken: "Open parent \($0.prefix(7))")
        }
        return CommitDetails(
            sha: sha, subject: commit.subject, spoken: "Details of \(commit.short), \(commit.subject), by \(commit.author)",
            message: commit.subject + (found.body.isEmpty ? "" : "\n\n" + found.body), authorLine: "\(commit.author) <\(email)>",
            people: [Fact("Author", [.init(commit.author), .init(" \(email)", .muted)])],
            parentsLabel: parents.count == 1 ? "Parent" : "Parents", parents: parents, refs: commit.refs, whole: found.body,
            changes: Fact("Changes", [.init("\(files.count) files "), .init("+\(added) ", .added), .init("\u{2212}\(removed)", .removed)]),
            files: files)
    }
}

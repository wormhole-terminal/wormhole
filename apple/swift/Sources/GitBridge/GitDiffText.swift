// A file's diff as the panel takes it: unified text for `UnifiedDiffParser`. Core decides what a diff is (text,
// binary, a Git LFS pointer, too large to load unasked, a submodule); the cases that have no lines to parse
// arrive as a `GitFailure` carrying core's own words, which the diff card shows, because `GitBackend.diff`
// returns text only. A richer return (a `DiffOutcome`) would give them their own looks.

import PanelModel
import WormholeBindings

extension GitDiff {
    func text() throws -> String {
        switch self {
        case let .text(patch), let .binary(patch, _): return patch
        case let .lfs(summary): throw GitFailure(["Stored with Git LFS", summary].compactMap { $0 }.joined(separator: ": "))
        case let .tooLarge(_, title, detail): throw GitFailure("\(title). \(detail)")
        case let .submodule(label, note, ahead, behind):
            let commits = (ahead.map { "+ \($0.short) \($0.subject)" } + behind.map { "\u{2212} \($0.short) \($0.subject)" })
            throw GitFailure(([label, note].compactMap { $0 } + commits).joined(separator: "\n"))
        }
    }
}

// A file's diff as the panel takes it: core's parsed hunks as `DiffFile`. Core decides what a diff is (text, binary,
// a Git LFS pointer, too large to load unasked, a submodule); the cases that have no lines arrive
// as a `DiffFile` with no lines and core's own words in `note`, which the card shows over the hatch.

import PanelModel
import WormholeBindings

extension DiffFile {
    init(_ file: GitDiffFile, isBinary: Bool = false) {
        let hunks = file.hunks.map { hunk in
            DiffHunk(header: hunk.header, lines: hunk.lines.map(DiffLine.init))
        }
        self.init(oldPath: file.oldPath, newPath: file.newPath, isBinary: isBinary, hunks: hunks)
    }
}

extension DiffLine {
    init(_ line: GitDiffLine) {
        let kind: Kind =
            switch line.kind {
            case .context: .context
            case .added: .added
            case .removed: .removed
            }
        self.init(kind: kind, oldNumber: line.old.map(Int.init), newNumber: line.new.map(Int.init), text: line.text, noNewline: line.noNewline)
    }
}

extension GitDiff {
    func file() throws -> DiffFile {
        switch self {
        case let .text(file): return DiffFile(file)
        case let .binary(file, _): return DiffFile(file, isBinary: true)
        case let .lfs(summary): return DiffFile(note: ["Stored with Git LFS", summary].compactMap { $0 }.joined(separator: ": "))
        case let .tooLarge(_, title, detail): return DiffFile(note: "\(title). \(detail)")
        case let .submodule(label, note, ahead, behind):
            let commits = (ahead.map { "+ \($0.short) \($0.subject)" } + behind.map { "\u{2212} \($0.short) \($0.subject)" })
            return DiffFile(note: ([label, note].compactMap { $0 } + commits).joined(separator: "\n"))
        }
    }
}

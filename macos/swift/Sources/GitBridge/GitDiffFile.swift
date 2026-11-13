// A file's diff as the panel takes it: core's parsed hunks as `DiffFile`. Core decides what a diff is (text, binary,
// a Git LFS pointer, too large to load unasked, a submodule); the cases that have no lines arrive
// as a `DiffFile` with no lines and core's own words (the column notes, the too-large card, a submodule's commits).

import PanelModel
import WormholeBindings

extension DiffFile {
    init(_ file: GitDiffFile, isBinary: Bool = false) {
        let hunks = file.hunks.map { hunk in
            DiffHunk(header: hunk.header, lines: hunk.lines.map(DiffLine.init))
        }
        self.init(oldPath: file.oldPath, newPath: file.newPath, isBinary: isBinary, hunks: hunks)
        modeChange = file.modeChange
        noHighlight = file.highlightSkipped
        let note = { (note: GitSideNote?) in note.map { SideNote(text: $0.text, detail: $0.detail, oid: $0.oid) } }
        notes = DiffNotes(split: file.notes.split.map(note), unified: note(file.notes.unified))
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
        self.init(kind: kind, oldNumber: line.old.map(Int.init), newNumber: line.new.map(Int.init), text: line.text, noNewline: line.noNewline, openLine: line.openLine.map(Int.init))
    }
}

extension GitDiff {
    func file() throws -> DiffFile {
        switch self {
        case let .text(file): return DiffFile(file)
        case let .binary(file, _): return DiffFile(file, isBinary: true)
        case let .lfs(file, _):
            var lfs = DiffFile(file)
            lfs.isLFS = true
            return lfs
        case let .tooLarge(_, title, detail):
            var large = DiffFile()
            large.tooLarge = TooLarge(title: title, detail: detail)
            return large
        case let .submodule(_, card):
            var module = DiffFile()
            module.submodule = SubmoduleCommits(
                old: card.old, new: card.new, nothing: card.nothing, inside: card.inside, ahead: card.ahead.map(Commit.init),
                behind: card.behind.map(Commit.init), aheadTotal: Int(card.aheadTotal), behindTotal: Int(card.behindTotal))
            return module
        }
    }
}

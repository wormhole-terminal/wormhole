// What happened to a file, with the letter the Changes rows end in. Words and letters are the real panel's
// (`wormhole_core::git::ChangeKind`, the row tooltip's `about`).

import WordsModel

public enum ChangeKind: String, Sendable, CaseIterable {
    case modified, added, deleted, renamed, untracked, conflicted

    public var letter: String {
        switch self {
        case .modified: "M"
        case .added: "A"
        case .deleted: "D"
        case .renamed: "R"
        case .untracked: "U"
        case .conflicted: "!"
        }
    }

    public var title: String {
        switch self {
        case .modified: Words.text("changes-kind-modified", fallback: "Modified")
        case .added: Words.text("changes-kind-added", fallback: "Added")
        case .deleted: Words.text("changes-kind-deleted", fallback: "Deleted")
        case .renamed: Words.text("changes-kind-renamed", fallback: "Renamed")
        case .untracked: Words.text("changes-kind-untracked", fallback: "Untracked")
        case .conflicted: Words.text("changes-kind-conflicted", fallback: "Conflicted")
        }
    }
}

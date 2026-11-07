// What happened to a file, with the letter the Changes rows end in. Words and letters are the real panel's
// (`wormhole_core::git::ChangeKind`, the row tooltip's `about`).

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

    public var title: String { rawValue.prefix(1).uppercased() + rawValue.dropFirst() }
}

// What a branch picker lists: a branch, a tag, a commit to compare with, or the stashes (History only).

import WordsModel

public struct Rev: Hashable, Identifiable, Sendable {
    public enum Kind: String, Sendable { case branch, tag, commit, stashes }

    /// What git is given: the branch or tag name, a hash.
    public var name: String
    public var kind: Kind
    /// A commit's subject, shown muted after its short hash.
    public var detail: String

    public var id: String { "\(kind.rawValue):\(name)" }

    public init(name: String, kind: Kind = .branch, detail: String = "") {
        self.name = name
        self.kind = kind
        self.detail = detail
    }

    public static let stashes = Rev(name: "stash", kind: .stashes)

    /// How the chip and the menu call it: the name, a commit by its short hash, the stashes by their word.
    public var title: String {
        switch kind {
        case .commit: String(name.prefix(7))
        case .stashes: Words.text("history-stashes", fallback: "Stashes")
        default: name
        }
    }
}

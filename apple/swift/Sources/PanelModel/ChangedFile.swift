// One entry of `git status`, as the Changes tab lists it.

public enum ChangeStatus: String, Sendable {
    case modified = "M"
    case added = "A"
    case deleted = "D"
    case renamed = "R"
    case untracked = "?"
}

public struct ChangedFile: Identifiable, Hashable, Sendable {
    public var path: String
    public var status: ChangeStatus
    public var staged: Bool
    public var added: Int
    public var removed: Int

    /// A file can be listed twice (staged and unstaged edits), so the section is part of the identity.
    public var id: String { (staged ? "staged:" : "unstaged:") + path }

    public init(path: String, status: ChangeStatus, staged: Bool, added: Int = 0, removed: Int = 0) {
        self.path = path
        self.status = status
        self.staged = staged
        self.added = added
        self.removed = removed
    }

    public var name: String { path.split(separator: "/").last.map(String.init) ?? path }

    /// The folder with its trailing slash, dimmed next to the name; empty at the repository root.
    public var folder: String {
        guard let slash = path.lastIndex(of: "/") else { return "" }
        return String(path[...slash])
    }
}

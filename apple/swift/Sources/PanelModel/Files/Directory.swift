// One folder's entries as the Files tab lists them (`wormhole_core::view::file_tree::dir`): folders first, then
// files, each group in natural order, names already cleaned of control and bidi characters, cut at a cap with a
// count of what was left out. The backend (core, through the FFI) produces it; the tree only keeps it.

public struct DiskEntry: Hashable, Sendable {
    public enum Kind: Sendable { case folder, file, link }

    /// The real name, what a path is made of.
    public var name: String
    /// The name as drawn.
    public var label: String
    public var kind: Kind

    public init(name: String, label: String? = nil, kind: Kind) {
        self.name = name
        self.label = label ?? name
        self.kind = kind
    }

    /// A real folder, never a link to one: the tree does not open a link.
    public var isFolder: Bool { kind == .folder }
}

public struct Directory: Equatable, Sendable {
    public var entries: [DiskEntry]
    /// Entries left out: past the cap, or past what was read at all.
    public var more: Int
    /// The reader stopped before the end of a huge folder, so `more` counts only what it saw.
    public var atLeast: Bool

    public init(entries: [DiskEntry], more: Int = 0, atLeast: Bool = false) {
        self.entries = entries
        self.more = more
        self.atLeast = atLeast
    }
}

/// Why a folder could not be listed.
public enum DirectoryFailure: Error, Sendable {
    case gone, denied, other
}

// One line of the Files tree (`wormhole_core::view::file_tree::tree::Row`): an entry, or a muted note about the
// folder it sits under.

public struct FileRow: Identifiable, Equatable, Sendable {
    public enum Kind: Equatable, Sendable {
        case folder(open: Bool)
        case file
        /// A link never opens, and what it points at is never looked at.
        case link
        /// "N more": entries the folder has that are not listed; `atLeast` when that is only what was seen.
        case more(Int, atLeast: Bool)
        case reading
        case empty
        case unreadable(DirectoryFailure)
    }

    /// The entry's path below the root; for a note, the folder's it is about.
    public var path: String
    public var depth: Int
    public var label: String
    public var kind: Kind

    public var id: String {
        switch kind {
        case .folder, .file, .link: path
        default: "\(path)\u{1f}note"
        }
    }

    public var isEntry: Bool {
        switch kind {
        case .folder, .file, .link: true
        default: false
        }
    }

    public var isFolder: Bool { if case .folder = kind { true } else { false } }

    public var name: String { path.split(separator: "/").last.map(String.init) ?? path }
}

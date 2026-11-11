// One entry of `git status` as the Changes and History lists show it. `added` is nil for a file without
// lines to count (binary), as the real row's "bin".

import WordsModel

public struct ChangedFile: Identifiable, Hashable, Sendable {
    public var path: String
    public var originalPath: String?
    public var kind: ChangeKind
    public var added: Int?
    public var removed: Int?
    public var lfs: Bool
    /// A submodule rather than a file: what the parent sees of it, in core's words ("2 new commits").
    public var submodule: String?
    /// The submodule is checked out and still there: its row offers a terminal in it.
    public var submoduleEnterable: Bool

    public var id: String { path }

    public init(
        path: String, kind: ChangeKind, added: Int? = 0, removed: Int? = 0, originalPath: String? = nil,
        lfs: Bool = false, submodule: String? = nil, submoduleEnterable: Bool = false
    ) {
        self.path = path
        self.kind = kind
        self.added = added
        self.removed = removed
        self.originalPath = originalPath
        self.lfs = lfs
        self.submodule = submodule
        self.submoduleEnterable = submoduleEnterable
    }

    public var name: String { path.split(separator: "/").last.map(String.init) ?? path }

    /// The folder without its trailing slash, as the row shows it beside the name; empty at the root.
    public var folder: String {
        guard let slash = path.lastIndex(of: "/") else { return "" }
        return String(path[..<slash])
    }

    /// What the row says of the lines: counts, "bin", "LFS" or a submodule's words.
    public enum Lines: Equatable, Sendable { case counts(added: Int, removed: Int), binary, lfs, submodule(String) }

    public var lines: Lines {
        if let submodule { return .submodule(submodule) }
        if lfs { return .lfs }
        guard let added else { return .binary }
        return .counts(added: added, removed: removed ?? 0)
    }

    /// A renamed file shows its old name when the name changed, and a mark when the folder did.
    public var renamedFrom: String? {
        guard kind == .renamed, let old = originalPath else { return nil }
        let oldName = String(old.split(separator: "/").last ?? "")
        return oldName != name ? oldName : nil
    }

    public var moved: Bool {
        guard kind == .renamed, let old = originalPath else { return false }
        return ChangedFile(path: old, kind: .renamed).folder != folder
    }

    /// The tooltip's second line: "Added · +2210", "Renamed from src/old.rs · +1 −1".
    public var about: String {
        let first = kind == .renamed
            ? originalPath.map { Words.text("changes-renamed-from", ["kind": .text(kind.title), "path": .text($0)], fallback: "\(kind.title) from \($0)") } ?? kind.title
            : kind.title
        let counts: String
        switch lines {
        case let .submodule(words): counts = words
        case .lfs: counts = "LFS" // text: Git LFS, a product name
        case .binary: counts = Words.text("changes-counts-binary", fallback: "binary")
        case let .counts(added, removed): counts = removed > 0 ? "+\(added) \u{2212}\(removed)" : "+\(added)"
        }
        return "\(first) · \(counts)"
    }
}

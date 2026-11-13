// The tree of folders a Files tab shows (`wormhole_core::view::file_tree::tree`): which folders were read and what
// they held, which are open, and the rows that makes. Paths are relative to the root, which is the empty path. A
// folder read again replaces what the tree knew of it; an open folder that is gone from its parent is dropped with
// everything under it. Reading itself is the backend's.

public struct FileTree: Equatable, Sendable {
    private enum Slot: Equatable, Sendable {
        case ready(Directory)
        case failed(DirectoryFailure)
    }

    private var slots: [String: Slot] = [:]
    private var open: Set<String> = []

    public init() {}

    public mutating func clear() {
        slots = [:]
        open = []
    }

    public func isOpen(_ path: String) -> Bool { open.contains(path) }

    /// Whether the tree holds an answer for the folder (its entries, or why it could not be read).
    public func isRead(_ path: String) -> Bool { slots[path] != nil }

    public func entry(_ path: String) -> DiskEntry? {
        guard case let .ready(dir) = slots[Self.parent(path)] else { return nil }
        return dir.entries.first { $0.name == Self.leaf(path) }
    }

    /// Opens a folder or closes it; whether it now needs reading. Only a real folder of a folder already read
    /// counts, which is where a link is never entered.
    @discardableResult
    public mutating func toggle(_ path: String) -> Bool {
        guard entry(path)?.isFolder == true else { return false }
        if open.remove(path) != nil { return false }
        open.insert(path)
        if case .ready = slots[path] { return false }
        return true
    }

    /// The folders to read now to keep what is shown true: the root and each open folder that shows, the ones
    /// above first.
    public var wanted: [String] {
        var found = [""]
        var at = 0
        while at < found.count {
            let path = found[at]
            at += 1
            guard case let .ready(dir) = slots[path] else { continue }
            for entry in dir.entries where entry.isFolder {
                let child = Self.join(path, entry.name)
                if open.contains(child) { found.append(child) }
            }
        }
        return found
    }

    /// The answer for `path`. Whether it changes what is shown: a folder read again that holds the same is no
    /// change, so a refresh that finds nothing new redraws nothing.
    @discardableResult
    public mutating func set(_ path: String, _ answer: Result<Directory, DirectoryFailure>) -> Bool {
        let slot: Slot
        switch answer {
        case let .success(dir): slot = .ready(dir)
        case let .failure(failure): slot = .failed(failure)
        }
        let shown = isShown(path)
        let changed = slots[path] != slot
        slots[path] = slot
        let pruned = prune()
        return (changed && shown) || pruned
    }

    /// The entry `from` renamed to `to` in the same folder: what the tree knew of a renamed folder moves with it, so
    /// it does not close.
    public mutating func renamed(_ from: String, to: String) {
        guard Self.parent(from) == Self.parent(to) else { return }
        if case var .ready(dir) = slots[Self.parent(from)], let at = dir.entries.firstIndex(where: { $0.name == Self.leaf(from) }) {
            dir.entries[at].name = Self.leaf(to)
            dir.entries[at].label = Self.leaf(to)
            slots[Self.parent(from)] = .ready(dir)
        }
        func moved(_ path: String) -> String {
            path == from ? to : path.hasPrefix(from + "/") ? to + path.dropFirst(from.count) : path
        }
        slots = Dictionary(uniqueKeysWithValues: slots.map { (moved($0.key), $0.value) })
        open = Set(open.map(moved))
    }

    /// The rows, top to bottom: each folder's entries and under an open folder its own. A folder that is empty,
    /// not read yet or can't be read is a note about it, the root's too.
    public var rows: [FileRow] {
        var rows: [FileRow] = []
        push("", depth: 0, into: &rows)
        return rows
    }

    private func push(_ folder: String, depth: Int, into rows: inout [FileRow]) {
        func note(_ kind: FileRow.Kind) -> FileRow { FileRow(path: folder, depth: depth, label: "", kind: kind) }
        switch slots[folder] {
        case nil: rows.append(note(.reading))
        case let .failed(failure): rows.append(note(.unreadable(failure)))
        case let .ready(dir):
            if dir.entries.isEmpty, dir.more == 0 { rows.append(note(.empty)) }
            for entry in dir.entries {
                let path = Self.join(folder, entry.name)
                let kind: FileRow.Kind =
                    switch entry.kind {
                    case .folder: .folder(open: open.contains(path))
                    case .file: .file
                    case .link: .link
                    }
                rows.append(FileRow(path: path, depth: depth, label: entry.label, kind: kind))
                if entry.isFolder, open.contains(path) { push(path, depth: depth + 1, into: &rows) }
            }
            if dir.more > 0 { rows.append(note(.more(dir.more, atLeast: dir.atLeast))) }
        }
    }

    private func isShown(_ path: String) -> Bool {
        var current = path
        while !current.isEmpty {
            guard open.contains(current) else { return false }
            current = Self.parent(current)
        }
        return true
    }

    /// Drops open folders and answers whose entry is gone from a folder that was read, with what was under them.
    private mutating func prune() -> Bool {
        func stale(_ path: String) -> Bool {
            guard !path.isEmpty, case let .ready(dir) = slots[Self.parent(path)] else { return false }
            return dir.entries.first { $0.name == Self.leaf(path) }?.isFolder != true
        }
        let gone = Set(slots.keys).union(open).filter(stale)
        guard !gone.isEmpty else { return false }
        let underGone = { (path: String) in gone.contains { path == $0 || path.hasPrefix($0 + "/") } }
        slots = slots.filter { !underGone($0.key) }
        open = open.filter { !underGone($0) }
        return true
    }

    public static func parent(_ path: String) -> String { path.lastIndex(of: "/").map { String(path[..<$0]) } ?? "" }
    public static func leaf(_ path: String) -> String { path.lastIndex(of: "/").map { String(path[path.index(after: $0)...]) } ?? path }
    public static func join(_ folder: String, _ name: String) -> String { folder.isEmpty ? name : folder + "/" + name }
}

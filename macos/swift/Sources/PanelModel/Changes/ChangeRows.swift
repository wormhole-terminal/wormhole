// Flattens the file list into rows: one card per section (just "Changes" without grouping), files flat or in a
// folder tree whose chains of single folders read as "a/b/c". A section over `foldAbove` files starts folded
// and says why under its head (`view::panel::section_folded`, `folded_note`).

import WordsModel

public struct ChangeRows: Sendable {
    public static let foldAbove = 200

    public var tree = false
    public var groupUntracked = true
    /// The sections folded or opened by the person: absent means "by size".
    public var folds: [ChangeGroup: Bool] = [:]
    /// Folders folded in the tree, by their path in the section.
    public var collapsed: Set<String> = []

    public init(tree: Bool = false, groupUntracked: Bool = true) {
        self.tree = tree
        self.groupUntracked = groupUntracked
    }

    public func build(_ files: [ChangedFile]) -> [ChangeRow] {
        let sections: [(ChangeGroup, [ChangedFile])] =
            groupUntracked
            ? [(.changes, files.filter { $0.kind != .untracked }), (.untracked, files.filter { $0.kind == .untracked })]
                .filter { !$0.1.isEmpty }
            : [(.all, files)]
        var rows: [ChangeRow] = []
        for (group, members) in sections {
            let folded = folds[group] ?? (members.count > Self.foldAbove)
            rows.append(.band(group: group, count: members.count, folded: folded, first: rows.isEmpty))
            if folded {
                if members.count > Self.foldAbove {
                    rows.append(.foldNote(Self.foldedNote(members.map(\.path), untracked: group == .untracked)))
                }
                continue
            }
            if tree {
                appendTree(Node(members), prefix: "", depth: 0, group: group, into: &rows)
            } else {
                rows += members.map { .file($0, depth: 0, inTree: false) }
            }
        }
        return rows + [.spacer]
    }

    /// The files in the order the list shows them, which is what the diff's previous and next step through.
    public func order(_ files: [ChangedFile]) -> [ChangedFile] {
        var open = self
        open.folds = [.all: false, .changes: false, .untracked: false]
        open.collapsed = []
        return open.build(files).compactMap { if case let .file(file, _, _) = $0 { file } else { nil } }
    }

    static func foldedNote(_ paths: [String], untracked: Bool) -> String {
        let why = Words.text("changes-folded", ["count": .text(CountText.grouped(foldAbove))], fallback: "Folded because there are more than \(CountText.grouped(foldAbove)).")
        var byFolder: [Substring: Int] = [:]
        for path in paths { if let top = path.split(separator: "/", maxSplits: 1).first, top.count != path.count { byFolder[top, default: 0] += 1 } }
        // The biggest folder, the first by name on a tie, so the note does not change between refreshes.
        guard let crowded = byFolder.max(by: { ($0.value, $1.key) < ($1.value, $0.key) }), crowded.value * 2 > paths.count
        else { return why }
        let count = CountText.grouped(crowded.value)
        guard untracked else {
            return Words.text(
                "changes-folded-crowded", ["count": .text(count), "folder": .text(String(crowded.key)), "why": .text(why)],
                fallback: "\(count) of them are in \(crowded.key)/. \(why)")
        }
        return Words.text(
            "changes-folded-crowded-untracked", ["count": .text(count), "folder": .text(String(crowded.key)), "why": .text(why)],
            fallback: "\(count) of them are in \(crowded.key)/, which isn't ignored. \(why)")
    }

    private func appendTree(_ node: Node, prefix: String, depth: Int, group: ChangeGroup, into rows: inout [ChangeRow]) {
        for name in node.dirs.keys.sorted() {
            guard var child = node.dirs[name] else { continue }
            var label = name
            while child.files.isEmpty, child.dirs.count == 1, let (next, only) = child.dirs.first {
                label += "/\(next)"
                child = only
            }
            let path = prefix.isEmpty ? label : "\(prefix)/\(label)"
            let isCollapsed = collapsed.contains("\(group.rawValue)\u{1f}\(path)")
            rows.append(.directory(group: group, path: path, label: label, depth: depth, collapsed: isCollapsed))
            if !isCollapsed { appendTree(child, prefix: path, depth: depth + 1, group: group, into: &rows) }
        }
        rows += node.files.map { .file($0, depth: depth, inTree: true) }
    }

    private struct Node {
        var dirs: [String: Node] = [:]
        var files: [ChangedFile] = []

        init(_ members: [ChangedFile]) {
            for file in members { insert(file, below: file.path.split(separator: "/").dropLast()) }
        }

        private mutating func insert(_ file: ChangedFile, below folders: ArraySlice<Substring>) {
            guard let head = folders.first else { return files.append(file) }
            dirs[String(head), default: Node([])].insert(file, below: folders.dropFirst())
        }
    }
}

extension ChangeGroup: Hashable {}

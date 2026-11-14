import Foundation

/// The sidebar as `rows` prints it (`AppController+Workspace.swift`): a header line, then a line per group head and per row.
///
///     selected=1 live=[1, 2] groups=1
///     group 1 "Build" folded=false pinned=false count=1 alerts=0 spoken="…"
///       row 2 "api" idle number=2 pinned=false folder=~/api line="…" trail=- split=[2] spoken="…"
struct Listing {
    struct Row {
        var id: Int
        var title: String
        var activity: String
        var number: Int?
        var folder: String
        var pinned = false
    }

    struct Group {
        var id: Int
        var name: String
        var folded: Bool
        var pinned = false
        /// Its terminals, a folded group's too.
        var count = 0
    }

    var selected: Int?
    var live: [Int] = []
    var groups: [Group] = []
    var rows: [Row] = []
    let text: String

    init(_ text: String) {
        self.text = text
        for line in text.split(separator: "\n") {
            let words = line.split(separator: " ")
            if line.hasPrefix("selected=") {
                selected = Self.value("selected=", in: line).flatMap { Int($0) }
                live = Self.integers(in: Self.between("live=", and: " groups=", in: String(line)))
            } else if words.first == "group", words.count > 2, let id = Int(words[1]) {
                groups.append(
                    Group(
                        id: id, name: Self.quoted(in: String(line)), folded: line.contains("folded=true"), pinned: line.contains("pinned=true"),
                        count: Self.value("count=", in: line).flatMap { Int($0) } ?? 0))
            } else if words.first == "row", words.count > 3, let id = Int(words[1]) {
                // The activity is the word after the closing quote of the title.
                let title = Self.quoted(in: String(line))
                let rest = line.components(separatedBy: "\"\(title)\" ").dropFirst().joined(separator: "\"\(title)\" ")
                let after = rest.split(separator: " ").map(String.init)
                rows.append(
                    Row(
                        id: id, title: title, activity: after.first ?? "",
                        number: Self.value("number=", in: Substring(rest)).flatMap { Int($0) },
                        folder: Self.value("folder=", in: Substring(rest)) ?? "", pinned: rest.contains(" pinned=true ")))
            }
        }
    }

    func row(_ id: Int) -> Row? { rows.first { $0.id == id } }

    private static func value(_ key: String, in line: Substring) -> String? {
        guard let range = line.range(of: key) else { return nil }
        return String(line[range.upperBound...].prefix { $0 != " " })
    }

    private static func between(_ start: String, and end: String, in line: String) -> String {
        guard let from = line.range(of: start) else { return "" }
        let tail = line[from.upperBound...]
        return String(tail[..<(tail.range(of: end)?.lowerBound ?? tail.endIndex)])
    }

    private static func integers(in text: String) -> [Int] {
        text.split { !$0.isNumber }.compactMap { Int($0) }
    }

    /// The first double-quoted run of the line (a name has no quote in these scenarios).
    private static func quoted(in line: String) -> String {
        let parts = line.split(separator: "\"", maxSplits: 2, omittingEmptySubsequences: false)
        return parts.count > 1 ? String(parts[1]) : ""
    }
}

/// `panes` (`WindowHub+Reading.swift`): `split=true layout=columns`, then `pane <id> <x> <y> <w> <h>[ focused][ zoomed]`.
struct Panes {
    var isSplit = false
    var ids: [Int] = []
    var focused: Int?
    /// Each pane's width and height, by id.
    var sizes: [Int: (width: Double, height: Double)] = [:]

    init(_ text: String) {
        for line in text.split(separator: "\n") {
            let words = line.split(separator: " ")
            if line.hasPrefix("split=") {
                isSplit = words.first == "split=true"
            } else if words.first == "pane", words.count > 1, let id = Int(words[1]) {
                ids.append(id)
                if words.contains("focused") { focused = id }
                if words.count > 5, let width = Double(words[4]), let height = Double(words[5]) { sizes[id] = (width, height) }
            }
        }
    }
}

// The History log under the keys (`features/git/history/keys.rs`): the commits, and under the open one its details and files.
// → and ← show or fold a commit's files, ↩ and Space do the same on a commit and open a file's diff, ⌘C copies the hash of
// the commit the keys are on. The list keeps the keyboard; a diff that is open follows the keys from file to file.

import PanelModel

extension HistoryStore: PanelWalking {
    public var walkRows: [WalkRow] {
        model.commits.flatMap { commit -> [WalkRow] in
            let open = commit.id == expanded
            var rows = [WalkRow(id: "commit-" + commit.id, level: 0, isOpen: open, name: commit.subject)]
            if open {
                rows.append(WalkRow(id: "details-" + commit.id, level: 1))
                rows += files.map { WalkRow(id: "file-" + $0.id, level: 1, name: $0.name) }
            }
            return rows
        }
    }

    public var following: Bool { selection != nil }

    public func press(_ row: WalkRow, space: Bool, following: Bool) {
        if row.id.hasPrefix("file-") {
            open(String(row.id.dropFirst(5)))
        } else if row.id.hasPrefix("commit-"), !following {
            fold(row)
        } else if row.id.hasPrefix("details-"), !following {
            // ↩ on the details: their menu under the row, the keyboard's way to the copies and the parents.
            detailsMenus += 1
        }
    }

    public func fold(_ row: WalkRow) {
        if row.id.hasPrefix("commit-") { toggle(String(row.id.dropFirst(7))) }
    }

    public func copy(_ row: WalkRow) -> String? { row.id.hasPrefix("commit-") ? String(row.id.dropFirst(7)) : nil }
}

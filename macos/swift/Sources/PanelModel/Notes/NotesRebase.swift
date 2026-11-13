// A merge with the project's notes.md is made from a copy of the notes, off the main actor, so the person may tick a step or
// start a note while it runs. The merge's answer is then applied to what the terminal holds now, as the GPUI app's is
// (`features/notes/api.rs`, on the UI thread): what changed meanwhile is kept on top of it, never overwritten by it.
//
// The three models are compared by what their items say, never by their ids: an id does not survive a load from the
// database or a merge, which both make new ones, so a comparison by id finds no item in common and adds each a second time.

import Foundation

extension NotesModel {
    /// `merged`, the answer to a merge of `sent`, with what was done to `sent` since (it is `now` by then): a step ticked or a
    /// note folded meanwhile stays as the person left it, one added meanwhile is added, one deleted or rewritten meanwhile
    /// stays so, and the file's own changes stay.
    public static func rebase(merged: NotesModel, sent: NotesModel, now: NotesModel) -> NotesModel {
        var result = merged
        result.tasks = TaskList(
            rebase(merged.tasks.items, sent: sent.tasks.items, now: now.tasks.items, text: \.text) { item, before, after in
                // Ticked or unticked meanwhile: the person's; otherwise what the file says.
                guard after.done != before.done else { return item }
                var task = item
                task.done = after.done
                return task
            })
        result.notes = rebase(merged.notes, sent: sent.notes, now: now.notes, text: \.text) { item, before, after in
            // How a note is folded is the person's, not the file's.
            guard after.collapsed != before.collapsed else { return item }
            var note = item
            note.collapsed = after.collapsed
            return note
        }
        // How the cards are folded is the person's, not the file's.
        (result.stepsCollapsed, result.notesCollapsed, result.showDone) = (now.stepsCollapsed, now.notesCollapsed, now.showDone)
        return result
    }

    /// An item as the three lists tell it apart: what it says, and which one of those it is (the second "call Ann").
    private struct Key: Hashable {
        var text: String
        var nth: Int
    }

    private static func keyed<Item>(_ items: [Item], text: (Item) -> String) -> [(key: Key, item: Item)] {
        var seen: [String: Int] = [:]
        return items.map { item in
            let said = text(item)
            let nth = seen[said, default: 0]
            seen[said] = nth + 1
            return (Key(text: said, nth: nth), item)
        }
    }

    private static func rebase<Item>(
        _ merged: [Item], sent: [Item], now: [Item], text: (Item) -> String,
        keep: (_ merged: Item, _ before: Item, _ after: Item) -> Item
    ) -> [Item] {
        let current = keyed(now, text: text)
        let before = Dictionary(keyed(sent, text: text).map { ($0.key, $0.item) }, uniquingKeysWith: { first, _ in first })
        let after = Dictionary(current.map { ($0.key, $0.item) }, uniquingKeysWith: { first, _ in first })
        // What the merge kept, as the person left it meanwhile; what the file added is new to both.
        var out: [(key: Key, item: Item)] = keyed(merged, text: text).compactMap { entry in
            guard let was = before[entry.key] else { return entry }
            // Deleted meanwhile, or rewritten (its new words are added below).
            guard let isNow = after[entry.key] else { return nil }
            return (entry.key, keep(entry.item, was, isNow))
        }
        // What the person added meanwhile goes after the item it followed, else before the one that followed it (a first
        // item rewritten stays first), else at the end.
        for (at, entry) in current.enumerated() where before[entry.key] == nil && !out.contains(where: { $0.key == entry.key }) {
            let place = { (other: (key: Key, item: Item)) in out.firstIndex { $0.key == other.key } }
            if let earlier = current[..<at].reversed().compactMap(place).first {
                out.insert(entry, at: earlier + 1)
            } else {
                out.insert(entry, at: current[(at + 1)...].compactMap(place).first ?? out.endIndex)
            }
        }
        return out.map(\.item)
    }
}

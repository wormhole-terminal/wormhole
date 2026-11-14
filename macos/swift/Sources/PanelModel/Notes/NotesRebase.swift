// A merge with the project's notes.md is made from a copy of the notes, off the main actor, so the person may tick, add,
// delete or move a step or a note while it runs. The merge's answer is then laid over what the terminal holds by then, so
// the person ends up where the GPUI app leaves them (`features/notes/api.rs` merges on the UI thread, before the next
// edit): the file's changes, with what the person did meanwhile on top.
//
// It is the core's three-way merge (`project::merge_notes`) for the one case it meets here: the base is what was sent, the
// person's side is what the terminal holds now, the other side is the merge's answer. So the same rules: items are told
// apart by what they say and which occurrence of it they are (the core's `match_exact`), never by their ids, which a load
// from the database and a merge both make anew; the order is the person's, and what the file added goes after the item it
// follows; a step the file removed stays when the person ticked it meanwhile, and a step the person deleted meanwhile comes
// back when the merge ticked or unticked it (core `merge.rs`: "deleting a task on one side removes it unless the other side
// changed it": the change is someone's work, the deletion can be made again). An item keeps the id it has on screen, so
// an editor opened during the merge stays on its row.

import Foundation

extension NotesModel {
    /// `merged`, the answer to a merge of `sent`, laid over `now`, what the terminal holds when it lands.
    public static func rebase(merged: NotesModel, sent: NotesModel, now: NotesModel) -> NotesModel {
        var result = now
        result.tasks = TaskList(
            rebase(merged.tasks.items, sent: sent.tasks.items, now: now.tasks.items, text: \.text,
                   renew: { PanelTask(text: $0.text, done: $0.done) },
                   keep: { merged, before, after in
                       // Ticked or unticked meanwhile: the person's, even when the file removed the step.
                       guard after.done == before.done else { return after }
                       guard let merged else { return nil }
                       var task = after
                       task.done = merged.done
                       return task
                   },
                   // Deleted meanwhile while the file ticked or unticked it: back, as the file has it.
                   revive: { merged, before in merged.done != before.done }))
        // A note's words are what tell it apart, and its fold is the person's: one the file kept stays as it is now. A note
        // the file rewrote says something else, so it is a new item here and comes back by itself.
        let words = { (note: Note) in note.text.trimmingCharacters(in: .whitespacesAndNewlines) }
        result.notes = rebase(
            merged.notes, sent: sent.notes, now: now.notes, text: words, renew: { Note(text: $0.text, collapsed: $0.collapsed) }
        ) { merged, _, after in merged == nil ? nil : after }
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

    /// `keep` decides an item that was there at the send and is still there now: given the merge's item of the same key
    /// (nil when the merge removed it), it returns the item to keep, or nil. `revive` decides one that was there at the send
    /// and was deleted meanwhile: true brings the merge's item back, as the merge changed it.
    private static func rebase<Item: Identifiable>(
        _ merged: [Item], sent: [Item], now: [Item], text: (Item) -> String, renew: (Item) -> Item,
        keep: (_ merged: Item?, _ before: Item, _ after: Item) -> Item?,
        revive: (_ merged: Item, _ before: Item) -> Bool = { _, _ in false }
    ) -> [Item] where Item.ID == UUID {
        let current = keyed(now, text: text)
        let answer = keyed(merged, text: text)
        let before = Dictionary(keyed(sent, text: text).map { ($0.key, $0.item) }, uniquingKeysWith: { first, _ in first })
        let theirs = Dictionary(answer.map { ($0.key, $0.item) }, uniquingKeysWith: { first, _ in first })
        // The person's list, in the person's order: what was added or rewritten meanwhile stays, the rest as `keep` says.
        var out: [(key: Key, item: Item)] = current.compactMap { entry in
            guard let was = before[entry.key] else { return entry }
            guard let kept = keep(theirs[entry.key], was, entry.item) else { return nil }
            return (entry.key, kept)
        }
        // What the merge brought (the file's lines) goes after the item it follows in the answer while that is still there,
        // else before the first one after it that is, else at the end (lines appended to the file stay last). One that was
        // there at the send and is gone now was deleted meanwhile: it stays so, unless the merge changed it (`revive`).
        var ids = Set(out.map(\.item.id))
        let place = { (other: (key: Key, item: Item)) in out.firstIndex { $0.key == other.key } }
        for (at, entry) in answer.enumerated()
        where (before[entry.key].map { revive(entry.item, $0) } ?? true) && !out.contains(where: { $0.key == entry.key }) {
            // Two items never share an id (a note the file edited carries the id of the one the person rewrote).
            let item = ids.contains(entry.item.id) ? renew(entry.item) : entry.item
            ids.insert(item.id)
            let after = at > 0 ? place(answer[at - 1]).map { $0 + 1 } : nil
            let index = after ?? answer[(at + 1)...].compactMap(place).first ?? out.endIndex
            out.insert((entry.key, item), at: index)
        }
        return out.map(\.item)
    }
}

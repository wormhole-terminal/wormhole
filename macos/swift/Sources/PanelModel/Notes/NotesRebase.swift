// A merge with the project's notes.md is made from a copy of the notes, off the main actor, so the person may tick a step or
// start a note while it runs. The merge's answer is then applied to what the terminal holds now, as the GPUI app's is
// (`features/notes/api.rs`, on the UI thread): what changed meanwhile is kept on top of it, never overwritten by it.

import Foundation

extension NotesModel {
    /// `merged`, the answer to a merge of `sent`, with what was done to `sent` since (it is `now` by then): a step or note changed
    /// or added meanwhile stays as the person left it, one deleted meanwhile stays deleted, and the file's own changes stay.
    public static func rebase(merged: NotesModel, sent: NotesModel, now: NotesModel) -> NotesModel {
        var result = merged
        result.tasks = TaskList(
            rebase(merged.tasks.items, sent: sent.tasks.items, now: now.tasks.items) { item, before, after in
                after != before ? after : item
            })
        result.notes = rebase(merged.notes, sent: sent.notes, now: now.notes) { item, before, after in
            // Text the person wrote meanwhile wins; a fold alone leaves the file's text.
            if after.text != before.text { return after }
            var note = item
            if after.collapsed != before.collapsed { note.collapsed = after.collapsed }
            return note
        }
        // How the cards are folded is the person's, not the file's.
        (result.stepsCollapsed, result.notesCollapsed, result.showDone) = (now.stepsCollapsed, now.notesCollapsed, now.showDone)
        return result
    }

    private static func rebase<Item: Identifiable & Equatable>(
        _ merged: [Item], sent: [Item], now: [Item], keep: (_ merged: Item, _ before: Item, _ after: Item) -> Item
    ) -> [Item] where Item.ID: Hashable {
        let before = Dictionary(sent.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
        let after = Dictionary(now.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
        // What the merge kept, as the person left it meanwhile; what the file added is new to both.
        var out: [Item] = merged.compactMap { item in
            guard let was = before[item.id] else { return item }
            guard let is_ = after[item.id] else { return nil }
            return keep(item, was, is_)
        }
        // What the person added meanwhile goes after the item it followed, else at the end.
        for (at, item) in now.enumerated() where before[item.id] == nil && !out.contains(where: { $0.id == item.id }) {
            let anchor = now[..<at].reversed().compactMap { previous in out.firstIndex { $0.id == previous.id } }.first
            out.insert(item, at: anchor.map { $0 + 1 } ?? out.endIndex)
        }
        return out
    }
}

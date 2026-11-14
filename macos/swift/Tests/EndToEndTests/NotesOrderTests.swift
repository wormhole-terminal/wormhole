import Foundation
import Testing

/// The Notes tab's order and its stash (P06): next steps and notes put in another place (the result of a drop; the drag's
/// gesture is a person's) and kept so, and a prompt typed in an agent's input box stashed into a note. Read with `notes`, the
/// store the tab draws (`notes=[…] steps=[…]`, in the list's order).
extension EndToEnd {
    /// Where each of `items` stands in `text` (its first appearance), in order: increasing when they are listed in that order.
    static func positions(of items: [String], in text: String) -> [Int] {
        items.map { item in text.range(of: item).map { text.distance(from: text.startIndex, to: $0.lowerBound) } ?? -1 }
    }

    // GPUI: core `view::reorder::move_item` (dropped on an item, the dragged one takes its place), the notes store writing each
    // item's position from the list's order (`view/reorder.rs` header).
    @Test("a next step and a note dropped in another place stand there, and are there in that order after a restart")
    @MainActor func stepsAndNotesReordered() async throws {
        try await Instance.with { app in
            await app.send(["tab notes", "task-add step-one", "task-add step-two", "task-add step-three", "note-add note-alpha", "note-add note-beta"])
            await app.until("notes") { $0.contains("step-three:false") && $0.contains("note-beta") }

            // The first step dropped on the third: the other two move up.
            app.expect(await app.reply("task-move 1 3"), contains: "task-move 1 3")
            // The second note dropped on the first.
            app.expect(await app.reply("note-move 2 1"), contains: "note-move 2 1")
            let moved = await app.reply("notes")
            let steps = Self.positions(of: ["step-two", "step-three", "step-one"], in: moved)
            app.expect(steps == steps.sorted() && !steps.contains(-1), "two, three, one:\n\(moved)")
            let notes = Self.positions(of: ["note-beta", "note-alpha"], in: moved)
            app.expect(notes == notes.sorted() && !notes.contains(-1), "beta before alpha:\n\(moved)")
            app.expect(await app.reply("task-move 1 9"), contains: "error:")

            try await app.restart()
            await app.send(["tab notes"])
            let back = await app.until("notes") { $0.contains("step-one") && $0.contains("note-alpha") }
            let kept = Self.positions(of: ["step-two", "step-three", "step-one"], in: back)
            app.expect(kept == kept.sorted() && !kept.contains(-1), "the order is kept:\n\(back)")
            let keptNotes = Self.positions(of: ["note-beta", "note-alpha"], in: back)
            app.expect(keptNotes == keptNotes.sorted() && !keptNotes.contains(-1), "the notes' order is kept:\n\(back)")
        }
    }

    // GPUI: `features/notes/note_ops.rs:324` (what is typed in the agent's input box, `Pty::grab_prompt`, becomes a note, not
    // opened for writing, and the terminal keeps the keys), core `contents::prompt_in` (the last `> ` line, without its marker).
    @Test("a prompt typed in an agent's input box, never sent, is stashed into a new note without its marker; the terminal keeps the keys")
    @MainActor func typedPromptStashed() async throws {
        try await Instance.with { app in
            // `cat` writes the line back as an agent's input box shows it: `> ` and what is typed.
            await app.emit("> fix the login bug")
            await app.until("screen") { $0.components(separatedBy: "> fix the login bug").count - 1 >= 2 }
            await app.send(["tab notes"])
            app.expect(await app.reply("stash"), contains: "stash: the prompt is in a new note")
            let notes = await app.until("notes") { $0.contains("fix the login bug") }
            app.expect(notes, contains: "notes=[\"fix the login bug\"]")
            app.expect((await app.reply("keys")).hasPrefix("focus=terminal"), "the terminal keeps the keys after a stash")
        }
    }
}

import CoreGraphics
import Foundation
import Testing

/// Find as a person uses it (T12): the bar opened with ⌘F and its query typed into the field by keys (`text`), matches painted
/// over the cells (read from the renderer's own picture, `shot`), ⌘G and ⇧⌘G, ↩ and ⇧↩ in the field, a match in the history
/// scrolled into view, the three toggles clicked where the accessibility tree puts them, an invalid pattern, output that keeps
/// arriving, and Esc giving the keys back. A real zsh prints the text, so a match is only what the program printed: the typed
/// command line holds the word split in two (`nee""dle`), as the shell reads it, and is no match.
extension EndToEnd {
    /// `N of M` of the bar's count as numbers.
    private static func count(_ readout: String) -> (at: Int, of: Int)? {
        guard let words = Readout(readout).first("find-state")?["count"] else { return nil }
        let numbers = words.split { !$0.isNumber }.compactMap { Int($0) }
        return numbers.count == 2 ? (numbers[0], numbers[1]) : nil
    }

    /// The centre of the control with this accessibility label, in the window's points.
    private static func centre(of label: String, in tree: String) -> CGPoint? {
        guard let line = tree.split(separator: "\n").first(where: { $0.contains("\"\(label)") && $0.contains(" @") }), let at = line.range(of: " @", options: .backwards) else { return nil }
        let numbers = line[at.upperBound...].split { $0 == "," || $0 == " " || $0 == "x" }.compactMap { Double($0) }
        return numbers.count == 4 ? CGPoint(x: numbers[0] + numbers[2] / 2, y: numbers[1] + numbers[3] / 2) : nil
    }

    @Test("Find: the query typed into the bar, matches painted, next and previous, the history scrolled to a match, toggles, output arriving, Esc")
    @MainActor func findTypedIntoTheBar() async throws {
        try await Instance.with(shell: .zsh) { app in
            // 120 lines with a match each, then three in capitals: more than a screen, so most matches are in the history.
            await app.shell(#"for i in {1..120}; do print "row-$i nee""dle"; done; for i in 1 2 3; do print "UP-$i NEE""DLE"; done"#)
            await app.screen { $0.contains("UP-3 NEEDLE") }

            // ⌘F opens the bar with nothing in it; the field has the keys, so the word is typed into it.
            await app.send(["key cmd-f"])
            let open = await app.until("find-state") { Readout($0).first("find-state")?.has("open") == true }
            app.expect(Readout(open).first("find-state")?["query"] == "", "an empty query: \(open)")
            await app.send(["text needle"])
            let typed = await app.until("find-state") { Self.count($0)?.of == 123 }
            app.expect(Readout(typed).first("find-state")?["query"] == "needle", "the typed word is the query: \(typed)")

            // Matches are painted: the rows on screen that hold the word have its cells in a colour of their own, the current one in
            // another; the cell before the word and the rows without it keep the background.
            guard let marked = await app.shot("find-marked") else { return }
            // The background: a cell far right of any text.
            let background = marked.background(row: 0, column: 90)
            let screen = (await app.reply("screen")).split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
            var tints: [RGB] = []
            for (row, text) in screen.enumerated() {
                guard let word = text.range(of: "needle", options: .caseInsensitive) else { continue }
                let column = text.distance(from: text.startIndex, to: word.lowerBound)
                tints.append(marked.background(row: row, column: column))
                app.expect(marked.background(row: row, column: column + 5) != background, "row \(row): the word's last cell is painted")
                app.expect(marked.background(row: row, column: column - 1) == background, "row \(row): the cell before the word is not")
            }
            app.expect(tints.count > 10, "the screen shows many matches: \(tints.count)")
            app.expect(Set(tints).count == 2, "matches wear one colour and the current one another: \(Set(tints))")
            app.expect(!Set(tints).contains(background), "no match keeps the background")
            let current = Dictionary(grouping: tints, by: { $0 }).filter { $0.value.count == 1 }.keys
            app.expect(current.count == 1, "exactly one match is the current one: \(Set(tints))")

            // ⌘G steps to the next match, round the end: from the last to the first, which is in the history and scrolled into view.
            let stepped = await app.send(["key cmd-g"])
            app.expect(stepped[0], contains: "findNext")
            let next = await app.until("find-state") { Self.count($0)?.at == 1 }
            app.expect(Self.count(next)?.of == 123, "the count stays: \(next)")
            let revealed = await app.until("screen") { $0.contains("row-1 needle") }
            app.expect(revealed, contains: "row-1 needle")
            app.expect(Int(Instance.value("offset", in: await app.reply("scrollback")) ?? "") ?? 0 > 0, "the view went back into the history")
            // ⇧⌘G steps back, to the last match, at the live screen again. (↩ and ⇧↩ in the field are SwiftUI's own key handling, which
            // a key from outside never reaches: MANUAL.md.)
            await app.send(["key shift-cmd-g"])
            await app.until("find-state") { Self.count($0)?.at == 123 }
            await app.until("scrollback") { Instance.value("offset", in: $0) == "0" }

            // Match case: only the 120 in lower case. The toggle is a button with its name; it is on once clicked.
            var tree = await app.reply("a11y")
            guard let matchCase = Self.centre(of: "Match case", in: tree), let wholeWord = Self.centre(of: "Whole word", in: tree), let regex = Self.centre(of: "Regular expression", in: tree)
            else { return app.fail("the three toggles in the tree:\n\(tree)") }
            await app.send(["click \(Int(matchCase.x)) \(Int(matchCase.y))"])
            let cased = await app.until("find-state") { Self.count($0)?.of == 120 }
            app.expect(Readout(cased).first("find-state")?.has("case") == true, "the toggle is on: \(cased)")
            // Whole word: `needl` is no word of any line (the typed command holds the word split, so it is no match either).
            await app.send(["key cmd-f", "text needl"])
            await app.until("find-state") { Self.count($0)?.of == 120 }
            await app.send(["click \(Int(wholeWord.x)) \(Int(wholeWord.y))"])
            let none = await app.until("find-state") { Readout($0).first("find-state")?["count"] == "No results" }
            app.expect(Readout(none).first("find-state")?.has("word") == true, "whole word is on: \(none)")
            await app.send(["click \(Int(wholeWord.x)) \(Int(wholeWord.y))"])
            await app.until("find-state") { Readout($0).first("find-state")?.has("word") == false && Self.count($0)?.of == 120 }
            // Regex: `row-1\d\d` is row-100 to row-120, 21 lines; a pattern that is no pattern says so.
            await app.send(["click \(Int(regex.x)) \(Int(regex.y))"])
            await app.send(["key cmd-f", #"text row-1\d\d"#])
            await app.until("find-state") { Self.count($0)?.of == 21 }
            await app.send(["key cmd-f", "text ("])
            let invalid = await app.until("find-state") { Readout($0).first("find-state")?["count"] == "Not a valid pattern" }
            app.expect(Readout(invalid).first("find-state")?.has("regex") == true, "regex is on: \(invalid)")
            await app.send(["click \(Int(regex.x)) \(Int(regex.y))", "click \(Int(matchCase.x)) \(Int(matchCase.y))", "key cmd-f", "text needle"])
            await app.until("find-state") { Self.count($0)?.of == 123 && Readout($0).first("find-state")?.has("case") == false }
            tree = ""

            // Output that keeps arriving: the count follows it, without a key.
            await app.shell(#"for i in {1..30}; do print "late-$i nee""dle"; sleep 0.05; done"#)
            let more = await app.until("find-state") { Self.count($0)?.of == 153 }
            app.expect(Self.count(more)?.of == 153, "the 30 lines printed while the bar was open are counted: \(more)")

            // Esc closes the bar and the keys are the terminal's again: what is typed next goes to the program, nothing stays painted.
            await app.send(["key esc"])
            let closed = await app.until("find-state") { Readout($0).first("find-state")?.has("closed") == true }
            await app.send(["text echo after-find", "key return"])
            await app.screen { $0.contains("echo after-find") }
            guard let plain = await app.shot("find-closed") else { return }
            let background2 = plain.background(row: 0, column: 90)
            app.expect(background2 == background, "the background is as it was")
            let after = (await app.reply("screen")).split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
            for (row, text) in after.enumerated() {
                guard let word = text.range(of: "needle", options: .caseInsensitive) else { continue }
                let column = text.distance(from: text.startIndex, to: word.lowerBound)
                app.expect(plain.background(row: row, column: column) == background, "row \(row): the match is not painted any more")
            }
        }
    }
}

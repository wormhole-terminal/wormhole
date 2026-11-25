#if WITH_UNIFFI
import AppKit
import Testing
import TerminalCore
import UniFFISource

@testable import TerminalMetal

/// An input method composing text in the view (IME): what the system's text input calls on the view, in the order an input
/// method calls it, on the real engine and a real PTY (`ByteBench`). The candidate window and the input method's own panel cannot
/// be driven (`macos/parity/DRIVING.md`, "What still needs a person"); what is checked is the view's side of it: what it keeps as
/// marked, what it draws, what it sends the program and when, and the place it gives for the candidate window.
@MainActor
@Suite(.serialized) struct InputMethodLiveTests {
    private static let anywhere = NSRange(location: NSNotFound, length: 0)

    private func compose(_ bench: ByteBench, _ text: String, caret: Int? = nil) {
        bench.live.view.setMarkedText(text, selectedRange: NSRange(location: caret ?? text.utf16.count, length: 0), replacementRange: Self.anywhere)
        bench.live.view.drawFrame(force: true)
    }

    private func picture(_ bench: ByteBench) -> RenderedImage { bench.live.view.renderer.renderToImage() }

    /// How many cells of row `row` hold something other than the picture `blank` had there: the part the composed text takes.
    private func changedColumns(_ bench: ByteBench, row: Int, from blank: RenderedImage) -> [Int] {
        let renderer = bench.live.view.renderer, now = picture(bench), height = renderer.cellPixels.height
        return (0..<renderer.gridSize.cols).filter { column in
            let left = renderer.left(column), width = renderer.left(column + 1) - left
            return (0..<height).contains { y in
                (0..<width).contains { x in now.pixel(left + x, row * height + y) != blank.pixel(left + x, row * height + y) }
            }
        }
    }

    @Test func composedTextIsDrawnAtTheCursorChangedAsTheMethodChangesItAndSentOnlyWhenCommitted() async throws {
        let bench = try ByteBench("")
        #expect(await bench.start())
        let row = try #require(bench.live.view.renderer.cursorCell).row, column = try #require(bench.live.view.renderer.cursorCell).column
        let blank = picture(bench)

        compose(bench, "に")
        #expect(bench.live.view.hasMarkedText() && bench.live.view.markedRange() == NSRange(location: 0, length: 1))
        // A wide character is two cells, underlined, from the cursor's cell; nothing else on the row changes (the caret, a beam,
        // is drawn at the end of the text, in the cell after).
        let first = changedColumns(bench, row: row, from: blank)
        #expect(first.contains(column) && first.contains(column + 1), "the character's two cells are drawn: \(first)")
        #expect(first.allSatisfy { $0 >= column && $0 <= column + 2 }, "nothing else changed: \(first)")

        compose(bench, "にほん")
        #expect(bench.live.view.markedRange() == NSRange(location: 0, length: 3), "changed in place: three characters")
        let three = changedColumns(bench, row: row, from: blank)
        #expect(three.contains(column + 5) && three.allSatisfy { $0 >= column && $0 <= column + 6 }, "six cells and the caret: \(three)")

        compose(bench, "にほ")
        #expect(bench.live.view.markedRange() == NSRange(location: 0, length: 2), "shortened again")
        #expect(changedColumns(bench, row: row, from: blank).allSatisfy { $0 <= column + 4 }, "the cells of the third character are given back")

        // Nothing has reached the program, and the commit sends what the method commits (not what was marked), once.
        bench.live.view.insertText("日本", replacementRange: Self.anywhere)
        #expect(!bench.live.view.hasMarkedText())
        await bench.expect(ByteBench.hex([0xe6, 0x97, 0xa5, 0xe6, 0x9c, 0xac]), "日本 as UTF-8, and nothing before it")
    }

    @Test func cancellingDropsTheComposedTextWithoutSendingAnything() async throws {
        let bench = try ByteBench("")
        #expect(await bench.start())
        let row = try #require(bench.live.view.renderer.cursorCell).row
        let blank = picture(bench)

        // The method cancels by marking an empty text.
        compose(bench, "にほ")
        compose(bench, "")
        #expect(!bench.live.view.hasMarkedText() && bench.live.view.markedRange().location == NSNotFound)
        #expect(changedColumns(bench, row: row, from: blank).isEmpty, "nothing is drawn any more")

        // A click while composing ends it, as the view tells the input method to drop it (`endComposition`). The click also gives the
        // view the keys (the cursor's cell is a filled block from then on), so the picture to compare with is one taken after a click.
        bench.live.view.mouseDown(with: bench.down(.leftMouseDown, row: 5, column: 5))
        bench.live.view.mouseUp(with: bench.down(.leftMouseUp, row: 5, column: 5))
        bench.live.view.drawFrame(force: true)
        let clicked = picture(bench)
        compose(bench, "にほ")
        bench.live.view.mouseDown(with: bench.down(.leftMouseDown, row: 5, column: 5))
        bench.live.view.mouseUp(with: bench.down(.leftMouseUp, row: 5, column: 5))
        #expect(!bench.live.view.hasMarkedText())
        bench.live.view.drawFrame(force: true)
        #expect(changedColumns(bench, row: row, from: clicked).isEmpty)

        // The system says the marked text is over without a commit (unmarkText). Apple documents that as accepting the text; the view
        // drops it and sends nothing, which is pinned here as it is: whether a real input method ever ends a composition that way
        // is for a person to try (MANUAL.md, IME).
        compose(bench, "かな")
        bench.live.view.unmarkText()
        #expect(!bench.live.view.hasMarkedText())
        await bench.expect("", "nothing was sent by any of the three")
    }

    @Test func theCandidateWindowIsPlacedAtTheComposedTextsCells() async throws {
        let bench = try ByteBench("")
        #expect(await bench.start())
        let view = bench.live.view, window = bench.live.window
        let cursor = try #require(view.renderer.cursorCell)
        let cell = bench.live.cell
        /// The cell `column` of the cursor's row, as a screen rectangle: worked out from the grid, not from the view's own answer.
        func screen(_ column: Int) -> NSRect {
            let inView = NSRect(x: MetalRenderer.padding + CGFloat(column) * cell.width, y: MetalRenderer.padding + CGFloat(cursor.row) * cell.height, width: cell.width, height: cell.height)
            return window.convertToScreen(view.convert(inView, to: nil))
        }
        // With nothing composed: the cursor's cell.
        #expect(view.firstRect(forCharacterRange: NSRange(location: 0, length: 0), actualRange: nil) == screen(cursor.column))
        // Composing "にほん": the window sits under the part the method is converting: the start is the first cell, the third
        // character begins four cells in (each wide character takes two).
        compose(bench, "にほん", caret: 0)
        #expect(view.firstRect(forCharacterRange: NSRange(location: 0, length: 1), actualRange: nil) == screen(cursor.column))
        #expect(view.firstRect(forCharacterRange: NSRange(location: 2, length: 1), actualRange: nil) == screen(cursor.column + 4))
        // The cursor moves with output: the window follows it.
        view.endComposition()
        bench.live.view.source.write(Data("xyz".utf8))
        #expect(await bench.draw { view.renderer.cursorCell?.column != cursor.column })
        let moved = try #require(view.renderer.cursorCell)
        #expect(moved != cursor)
        #expect(view.firstRect(forCharacterRange: NSRange(location: 0, length: 0), actualRange: nil).origin.x == window.convertToScreen(view.convert(NSRect(x: MetalRenderer.padding + CGFloat(moved.column) * cell.width, y: 0, width: 1, height: 1), to: nil)).origin.x)
    }

    @Test func theConvertedClauseIsMarkedApartFromTheRestOfTheComposedText() async throws {
        let bench = try ByteBench("")
        #expect(await bench.start())
        let row = try #require(bench.live.view.renderer.cursorCell).row, column = try #require(bench.live.view.renderer.cursorCell).column
        let blank = picture(bench)
        let height = bench.live.view.renderer.cellPixels.height
        bench.live.view.setMarkedText("abc", selectedRange: NSRange(location: 1, length: 1), replacementRange: Self.anywhere)
        bench.live.view.drawFrame(force: true)
        let now = picture(bench)
        // The cell of the clause "b" has a background of its own (the selection's), the cells of "a" and "c" the screen's.
        func corner(_ offset: Int) -> (r: UInt8, g: UInt8, b: UInt8) {
            now.pixel(bench.live.view.renderer.left(column + offset), row * height + 1)
        }
        let background = blank.pixel(bench.live.view.renderer.left(column + 10), row * height + 1)
        #expect(corner(0) == background && corner(2) == background, "a and c")
        #expect(corner(1) != background, "b, the part the method is converting")
    }
}
#endif

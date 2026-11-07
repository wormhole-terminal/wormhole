import AppKit
import Testing
import TerminalCore

@testable import TerminalMetal

@Suite struct CellGridTests {
    private let grid = CellGrid(cellWidth: 8, cellHeight: 18, columns: 80, rows: 24)

    @Test func aPointIsACellAndWhichHalf() {
        #expect(grid.hit(CGPoint(x: 41, y: 20)).map { ($0.row, $0.column, $0.side) }! == (1, 5, .left))
        #expect(grid.hit(CGPoint(x: 45, y: 20)).map { ($0.row, $0.column, $0.side) }! == (1, 5, .right))
        #expect(grid.hit(CGPoint(x: 41, y: 20))?.x == 41 && grid.hit(CGPoint(x: 41, y: 20))?.y == 20, "pixels from the corner")
    }

    @Test func aSizedRowsCellsAreTwiceAsWideAndItShowsHalfItsColumns() {
        var sized = grid
        sized.rowSizes = [.single, .doubleWidth, .doubleHeightTop, .doubleHeightBottom]
        #expect(sized.hit(CGPoint(x: 41, y: 20)).map { ($0.row, $0.column, $0.side) }! == (1, 2, .right), "41 pt is cell 5 on a normal row, 2 here")
        #expect(sized.hit(CGPoint(x: 41, y: 2)).map { ($0.row, $0.column) }! == (0, 5), "the first row is a normal one")
        #expect(sized.hit(CGPoint(x: 41, y: 40)).map { ($0.row, $0.column) }! == (2, 2))
        #expect(sized.hit(CGPoint(x: 9000, y: 20)).map(\.column)! == 39, "clamped to the last column it shows")
    }

    @Test func aPointOutsideIsClampedToTheNearestCell() {
        #expect(grid.hit(CGPoint(x: -5, y: -5)).map { ($0.row, $0.column) }! == (0, 0))
        #expect(grid.hit(CGPoint(x: 9000, y: 9000)).map { ($0.row, $0.column) }! == (23, 79))
        #expect(CellGrid(cellWidth: 8, cellHeight: 18, columns: 0, rows: 0).hit(.zero) == nil)
    }

    /// The app's rule (`terminal_view/input.rs`): the distance past the edge in cells, beside the grid 0.
    @Test func dragsPastTheEdgesScrollByTheDistance() {
        #expect(grid.dragScroll(CGPoint(x: 50, y: 100)) == nil, "over the grid")
        #expect(grid.dragScroll(CGPoint(x: 50, y: -1)) == 1)
        #expect(grid.dragScroll(CGPoint(x: 50, y: -37)) == 3)
        let bottom = CGFloat(18 * 24)
        #expect(grid.dragScroll(CGPoint(x: 50, y: bottom)) == -1)
        #expect(grid.dragScroll(CGPoint(x: 50, y: bottom + 20)) == -2)
        #expect(grid.dragScroll(CGPoint(x: -2, y: 100)) == 0, "beside the grid")
    }
}

@Suite struct MarkedTextTests {
    @Test func theCaretAndClauseAreUTF16OffsetsAndTheRendererCountsCharacters() {
        var model = MarkedTextModel()
        model.set("a😀にほん", selection: NSRange(location: 3, length: 2))
        #expect(model.markedRange == NSRange(location: 0, length: 6))
        #expect(model.selectedRange == NSRange(location: 3, length: 2))
        // The emoji is two UTF-16 units but one character.
        #expect(model.drawn == MarkedText(text: "a😀にほん", caret: 2, clause: 2))
    }

    @Test func nothingComposedHasAValidEmptySelectionForDictation() {
        let model = MarkedTextModel()
        #expect(model.markedRange.location == NSNotFound && model.selectedRange == NSRange(location: 0, length: 0))
        #expect(model.substring(in: NSRange(location: 0, length: 3)) == nil && model.drawn == nil)
    }

    @Test func substringsAreClampedToWhatIsComposed() {
        var model = MarkedTextModel()
        model.set("日本語", selection: NSRange(location: NSNotFound, length: 0))
        #expect(model.selectedRange.location == 3, "no caret given: at the end")
        let part = model.substring(in: NSRange(location: 1, length: 10))
        #expect(part?.text == "本語" && part?.actual == NSRange(location: 1, length: 2))
    }

    @Test func theCandidateWindowSitsWhereTheComposedTextStartsThatManyCellsIn() {
        var model = MarkedTextModel()
        model.set("日本ab", selection: NSRange(location: 4, length: 0))
        #expect(model.columns(before: 0) == 0 && model.columns(before: 2) == 4 && model.columns(before: 3) == 5)
    }
}

@Suite struct ShortcutTests {
    private func shortcut(_ characters: String, _ flags: NSEvent.ModifierFlags, code: UInt16 = 0) -> TerminalShortcut? {
        TerminalShortcut.of(characters: characters, keyCode: code, flags: flags)
    }

    @Test func theTerminalsCommandKeys() {
        #expect(shortcut("c", .command) == .copy && shortcut("c", [.command, .shift]) == .copy)
        #expect(shortcut("v", .command) == .paste && shortcut("a", .command) == .selectAll && shortcut("k", .command) == .clear)
        #expect(shortcut("\u{7F}", .command, code: 51) == .send([0x15]))
        #expect(shortcut("\u{F702}", .command, code: 123) == .send([0x01]) && shortcut("\u{F703}", .command, code: 124) == .send([0x05]))
    }

    @Test func everythingElseIsTheApplications() {
        #expect(shortcut("t", .command) == nil && shortcut("z", .command) == nil)
        #expect(shortcut("a", [.command, .shift]) == nil, "⇧⌘A is an app key")
        #expect(shortcut("c", [.command, .option]) == nil && shortcut("c", [.command, .control]) == nil)
        #expect(shortcut("c", []) == nil, "no Command: a key for the program")
    }
}

@Suite struct KeyTableTests {
    @Test func theNamedKeysIncludeTheFunctionKeysTo20() {
        func name(_ code: UInt16) -> String? {
            let event = NSEvent.keyEvent(
                with: .keyDown, location: .zero, modifierFlags: [], timestamp: 0, windowNumber: 0, context: nil, characters: "\u{F704}",
                charactersIgnoringModifiers: "\u{F704}", isARepeat: false, keyCode: code)
            return event.flatMap { KeyMap.input(from: $0, optionAsMeta: .off)?.key }
        }
        #expect([name(122), name(111), name(105), name(90)] == ["f1", "f12", "f13", "f20"])
        #expect(name(36) == "enter" && name(76) == "enter" && name(51) == "backspace" && name(117) == "delete")
    }

    @Test func optionIsMetaOnTheSideTheSettingNames() {
        func input(_ flags: UInt, _ mode: OptionAsMeta) -> KeyInput? {
            let event = NSEvent.keyEvent(
                with: .keyDown, location: .zero, modifierFlags: NSEvent.ModifierFlags(rawValue: flags), timestamp: 0, windowNumber: 0,
                context: nil, characters: "∫", charactersIgnoringModifiers: "b", isARepeat: false, keyCode: 11)
            return event.flatMap { KeyMap.input(from: $0, optionAsMeta: mode) }
        }
        let option = NSEvent.ModifierFlags.option.rawValue
        let (left, right) = (option | 0x20, option | 0x40)
        #expect(input(left, .left)?.optionAsMeta == true && input(right, .left)?.optionAsMeta == false)
        #expect(input(right, .left)?.text == "∫", "the other Option types the layout's character")
        #expect(input(right, .right)?.optionAsMeta == true && input(left, .right)?.optionAsMeta == false)
        #expect(input(left, .both)?.optionAsMeta == true && input(right, .both)?.optionAsMeta == true)
        #expect(input(left, .off)?.optionAsMeta == false)
        #expect(input(option, .left)?.optionAsMeta == true, "a synthetic event with no side: any setting but off")
        #expect(input(option, .off)?.optionAsMeta == false)
    }
}

@MainActor
@Suite struct ScrollerMappingTests {
    @Test func theThumbIsTheShareOfTheScreenAndItsPositionIsHowFarBack() {
        let scroller = TerminalScroller()
        scroller.update(history: 80, rows: 20, offset: 0)
        #expect(scroller.view.isEnabled && abs(scroller.view.knobProportion - 0.2) < 1e-9 && scroller.view.doubleValue == 1, "live: at the bottom")
        scroller.update(history: 80, rows: 20, offset: 80)
        #expect(scroller.view.doubleValue == 0, "all the way back: at the top")
        scroller.update(history: 0, rows: 20, offset: 0)
        #expect(!scroller.view.isEnabled, "nothing to scroll")
    }

    @Test func aTrackClickPagesAndAThumbDragFollowsTheThumb() {
        func offset(_ part: NSScroller.Part, value: Double = 0, current: Int = 30) -> Int {
            TerminalScroller.offset(hit: part, value: value, history: 100, rows: 20, current: current)
        }
        #expect(offset(.decrementPage) == 49, "above the thumb: a page back, keeping a line")
        #expect(offset(.incrementPage) == 11)
        #expect(offset(.decrementPage, current: 95) == 100 && offset(.incrementPage, current: 5) == 0, "clamped to the history")
        #expect(offset(.knob, value: 0.25) == 75 && offset(.knob, value: 1) == 0)
    }
}

@Suite struct RowMarksTests {
    private let red = PackedColor.named(1)
    /// A palette whose theme sets no selection text colour (`keepsTextInSelection`), and one that does.
    private let keeping = TerminalPalette.spike
    private let themed = TerminalPalette(
        ansi: TerminalPalette.spike.slots[0..<16].map { $0 }, foreground: SIMD3(200, 200, 200), background: SIMD3(0, 0, 0),
        cursor: SIMD3(255, 255, 255), selection: SIMD3(40, 40, 90), selectionText: SIMD3(255, 255, 255))

    @Test func aSelectedCellIsTheSelectionOverTheCellAsItWouldHaveBeenDrawn() {
        var cells = [PackedCell(scalar: 0x61, fg: PackedColor.named(1), bg: PackedColor.named(2), flags: .inverse), PackedCell(scalar: 0x62)]
        RowMarks(selected: 0...0).apply(to: &cells, palette: keeping)
        #expect(cells[0].bg == PackedColor.selectionBackground)
        #expect(cells[0].fg == PackedColor.named(2), "inverse resolved: the text is the cell's background")
        #expect(!cells[0].cellFlags.contains(.inverse))
        #expect(cells[1] == PackedCell(scalar: 0x62), "outside the selection: untouched")
    }

    @Test func aThemeWithSelectionTextColourPaintsTheTextInItButHiddenTextStaysHidden() {
        var cells = [PackedCell(scalar: 0x61, fg: red), PackedCell(scalar: 0x62, fg: red, flags: .hidden)]
        RowMarks(selected: 0...1).apply(to: &cells, palette: themed)
        #expect(cells[0].fg == PackedColor.selectionForeground)
        #expect(cells[1].fg == PackedColor.background, "hidden text is the background's colour, not the theme's text colour")
    }

    @Test func aHoveredLinkIsUnderlinedUnlessTheProgramAlreadyUnderlinedIt() {
        var cells = [PackedCell(scalar: 0x61), PackedCell(scalar: 0x62, flags: .dottedUnderline), PackedCell(scalar: 0x63)]
        RowMarks(link: 0...1).apply(to: &cells, palette: keeping)
        #expect(cells[0].cellFlags.contains(.underline) && !cells[2].cellFlags.contains(.underline))
        #expect(cells[1].cellFlags == .dottedUnderline, "the program's own style stays")
    }
}

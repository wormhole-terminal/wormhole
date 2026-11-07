import AppKit
import Testing
import TerminalCore

@testable import TerminalMetal

@MainActor
@Suite struct PointerTests {
    private static let text = "hello world\rsee https://example.com/x now\rthird line\r"
    private let selectionColour = TerminalPalette.spike.selectionBackground

    private func isSelected(_ harness: ViewHarness, row: Int, column: Int) -> Bool {
        let p = harness.pixel(row: row, column: column)
        return (p.r, p.g, p.b) == (selectionColour.x, selectionColour.y, selectionColour.z)
    }

    @Test func draggingSelectsCellsAndTheRendererDrawsThemInTheSelectionColour() throws {
        let harness = try ViewHarness(text: Self.text)
        harness.drag(from: harness.point(row: 0, column: 2), to: harness.point(row: 0, column: 6))
        #expect(harness.interaction.selectionText() == "llo w")
        #expect(isSelected(harness, row: 0, column: 2) && isSelected(harness, row: 0, column: 6))
        #expect(!isSelected(harness, row: 0, column: 1) && !isSelected(harness, row: 0, column: 7))
        #expect(!isSelected(harness, row: 1, column: 3))
    }

    @Test func aClickWithoutADragSelectsNothingAndTwoClicksTakeAWord() throws {
        let harness = try ViewHarness(text: Self.text)
        harness.drag(from: harness.point(row: 0, column: 2), to: harness.point(row: 0, column: 2))
        #expect(harness.interaction.selectionText() == nil)
        harness.drag(from: harness.point(row: 0, column: 7), to: harness.point(row: 0, column: 7), clicks: 2)
        #expect(harness.interaction.selectionText() == "world")
        harness.drag(from: harness.point(row: 2, column: 1), to: harness.point(row: 2, column: 1), clicks: 3)
        #expect(harness.interaction.selectionText() == "third line")
    }

    @Test func optionDragIsARectangle() throws {
        let harness = try ViewHarness(text: Self.text)
        harness.drag(from: harness.point(row: 0, column: 1), to: harness.point(row: 2, column: 3), flags: .option)
        #expect(harness.interaction.selectionText() == "ell\nee\nhir")
    }

    @Test func aProgramThatTookTheMouseGetsTheEventsAndShiftTakesThemBack() throws {
        let harness = try ViewHarness(text: Self.text)
        harness.interaction.programTakesPointer = true
        harness.drag(from: harness.point(row: 0, column: 2), to: harness.point(row: 1, column: 4))
        let seen = harness.interaction.reportedPointer
        #expect(seen.map(\.action) == [.press, .motion, .release])
        #expect(seen.allSatisfy { $0.button == .left })
        #expect((seen[0].row, seen[0].column) == (0, 2) && (seen[2].row, seen[2].column) == (1, 4))
        #expect(harness.interaction.selectionText() == nil)
        harness.drag(from: harness.point(row: 0, column: 2), to: harness.point(row: 0, column: 5), flags: .shift)
        #expect(harness.interaction.selectionText() == "llo", "shift: the view selects")
        #expect(harness.interaction.reportedPointer.count == 3)
    }

    @Test func movingReportsHoverToAProgramThatAsked() throws {
        let harness = try ViewHarness(text: Self.text)
        harness.interaction.programTakesPointer = true
        harness.view.mouseMoved(with: harness.mouse(.mouseMoved, at: harness.point(row: 1, column: 3)))
        #expect(harness.interaction.reportedPointer.last.map { ($0.button, $0.action, $0.row, $0.column) }! == (.none, .motion, 1, 3))
    }

    @Test func commandClickOpensALinkAndCommandHoverUnderlinesIt() throws {
        let harness = try ViewHarness(text: Self.text)
        var opened: [OpenTarget] = []
        harness.view.linkOpener = { opened.append($0) }
        let onLink = harness.point(row: 1, column: 8)
        harness.view.mouseMoved(with: harness.mouse(.mouseMoved, at: onLink, flags: .command))
        harness.view.drawFrame(force: true)
        let underline = harness.view.renderer.renderToImage()
        harness.view.mouseMoved(with: harness.mouse(.mouseMoved, at: onLink))
        harness.view.drawFrame(force: true)
        let plain = harness.view.renderer.renderToImage()
        #expect(underline.bgra != plain.bgra, "command held: the link is underlined; released: it is not")

        harness.view.mouseDown(with: harness.mouse(.leftMouseDown, at: onLink, flags: .command))
        #expect(opened == [.url("https://example.com/x")])
        #expect(harness.interaction.selectionText() == nil, "the click opened the link instead of starting a selection")
        harness.view.mouseDown(with: harness.mouse(.leftMouseDown, at: harness.point(row: 0, column: 1), flags: .command))
        #expect(opened.count == 1, "no link there: the click does what it would")
    }

    @Test func aTrackpadScrollBecomesCellsAndAWheelIsLines() throws {
        let harness = try ViewHarness(text: Self.text)
        let precise = try #require(CGEvent(scrollWheelEvent2Source: nil, units: .pixel, wheelCount: 2, wheel1: 0, wheel2: 0, wheel3: 0))
        precise.setDoubleValueField(.scrollWheelEventPointDeltaAxis1, value: Double(harness.cell.height) * 3)
        precise.setDoubleValueField(.scrollWheelEventIsContinuous, value: 1)
        let event = try #require(NSEvent(cgEvent: precise))
        harness.view.scrollWheel(with: event)
        let step = try #require(harness.interaction.wheels.last)
        #expect(abs(step.lines - 3) < 0.05, "points divided by the cell's height: \(step.lines)")
    }

    @Test func draggingBelowTheGridScrollsTheHistoryAndKeepsSelecting() throws {
        let harness = try ViewHarness(rows: 6, text: (0..<30).map { "line \($0)\r" }.joined())
        let bottom = harness.cell.height * 6
        harness.view.mouseDown(with: harness.mouse(.leftMouseDown, at: harness.point(row: 3, column: 1)))
        harness.view.mouseDragged(with: harness.mouse(.leftMouseDragged, at: CGPoint(x: 10, y: bottom + harness.cell.height * 2.5)))
        #expect(harness.view.pointer.autoscroll != nil, "a timer keeps it going while the pointer stays out there")
        harness.view.mouseUp(with: harness.mouse(.leftMouseUp, at: CGPoint(x: 10, y: bottom + 5)))
        #expect(harness.view.pointer.autoscroll == nil)
    }

    @Test func theContextMenuHasTheAppsItemsAndAProgramThatHasTheMouseKeepsTheClick() throws {
        let harness = try ViewHarness(text: Self.text)
        let event = harness.mouse(.rightMouseDown, at: harness.point(row: 1, column: 8))
        let titles = try #require(harness.view.menu(for: event)).items.map(\.title).filter { !$0.isEmpty }
        #expect(titles == ["Open Link", "Copy Link", "Copy", "Paste", "Select All", "Find…", "Split Right", "Split Down", "Clear Buffer"])
        let plain = try #require(harness.view.menu(for: harness.mouse(.rightMouseDown, at: harness.point(row: 0, column: 1)))).items.map(\.title)
        #expect(!plain.contains("Open Link"))
        harness.interaction.programTakesPointer = true
        #expect(harness.view.menu(for: event) == nil)
        #expect(harness.view.menu(for: harness.mouse(.rightMouseDown, at: harness.point(row: 1, column: 8), flags: .shift)) != nil)
    }

    @Test func copyPutsTheSelectionOnThePasteboardAndSaysSo() throws {
        let harness = try ViewHarness(text: Self.text)
        harness.drag(from: harness.point(row: 0, column: 0), to: harness.point(row: 0, column: 4))
        var said: String?
        harness.view.onCopied = { said = $0 }
        let saved = NSPasteboard.general.string(forType: .string)
        defer {
            NSPasteboard.general.clearContents()
            if let saved { NSPasteboard.general.setString(saved, forType: .string) }
        }
        harness.view.copy(nil)
        #expect(said == "hello" && NSPasteboard.general.string(forType: .string) == "hello")
        #expect(harness.view.validateUserInterfaceItem(MenuProbe(#selector(MetalTerminalView.copy(_:)))))
    }

    @Test func pastingTextAndDroppedFilesGoToTheProgram() throws {
        let harness = try ViewHarness()
        let board = NSPasteboard(name: NSPasteboard.Name("wormhole.swift.test.\(UUID().uuidString)"))
        board.clearContents()
        board.setString("pasted", forType: .string)
        #expect(harness.view.insert(from: board))
        board.clearContents()
        board.writeObjects([URL(fileURLWithPath: "/tmp/My File.png") as NSURL])
        #expect(harness.view.insert(from: board))
        harness.view.drawFrame(force: true)
        let screen = harness.view.renderer.text(ofRow: 0)
        #expect(screen.hasPrefix("pasted/tmp/My\\ File.png "), "\(screen)")
    }
}

@MainActor
private final class MenuProbe: NSObject, NSValidatedUserInterfaceItem {
    let action: Selector?
    init(_ action: Selector) { self.action = action }
    var tag: Int { 0 }
}

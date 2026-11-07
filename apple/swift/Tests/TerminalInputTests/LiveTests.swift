#if WITH_UNIFFI
import AppKit
import Testing
import TerminalCore
import UniFFISource

@testable import TerminalMetal

/// The same view on the real engine and a PTY: a shell script prints what a test needs and `cat` stays as the
/// program that reads input (the tty echoes what is written to it, ESC as `^[`). Needs the FFI linked.
@MainActor
final class LiveHarness {
    let view: MetalTerminalView
    let window: NSWindow
    let cell: CGSize

    init(_ script: String, cols: Int = 40, rows: Int = 10) throws {
        setenv("WORMHOLE_DATA_DIR", "/tmp/Wormhole-swift-g-term-input", 1)
        let configuration = RendererConfiguration(font: .system(family: "Menlo"), pointSize: 13, scale: 2)
        let probe = try MetalRenderer(configuration: configuration)
        cell = CGSize(width: CGFloat(probe.cellPixels.width) / 2, height: CGFloat(probe.cellPixels.height) / 2)
        let frame = NSRect(x: 0, y: 0, width: cell.width * CGFloat(cols), height: cell.height * CGFloat(rows))
        let source = UniFFIFrameSource(program: "/bin/sh", arguments: ["-c", "\(script); exec cat"])
        view = try MetalTerminalView(source: source, configuration: configuration, frame: frame)
        view.fixedGrid = (cols, rows)
        window = NSWindow(contentRect: frame, styleMask: .borderless, backing: .buffered, defer: false)
        window.contentView = view
        view.layoutSubtreeIfNeeded()
    }

    deinit { MainActor.assumeIsolated { view.shutdown() } }

    func screen() -> String { (0..<view.renderer.gridSize.rows).map { view.renderer.text(ofRow: $0) }.joined(separator: "\n") }

    /// Draw frames until the screen shows `needle`.
    func wait(for needle: String) async -> Bool {
        for _ in 0..<250 {
            view.drawFrame(force: true)
            if screen().contains(needle) { return true }
            await withCheckedContinuation { done in DispatchQueue.main.asyncAfter(deadline: .now() + 0.02) { done.resume() } }
        }
        return false
    }

    /// `half` 0.25 is the left half of the cell, 0.75 the right: a selection includes a cell from the half it started in.
    func point(row: Int, column: Int, half: CGFloat = 0.5) -> CGPoint {
        CGPoint(x: (CGFloat(column) + half) * cell.width, y: (CGFloat(row) + 0.5) * cell.height)
    }

    func mouse(_ type: NSEvent.EventType, _ point: CGPoint, clicks: Int = 1, flags: NSEvent.ModifierFlags = []) -> NSEvent {
        NSEvent.mouseEvent(
            with: type, location: view.convert(point, to: nil), modifierFlags: flags, timestamp: 0, windowNumber: window.windowNumber,
            context: nil, eventNumber: 0, clickCount: clicks, pressure: 1)!
    }

    func drag(from: CGPoint, to: CGPoint, clicks: Int = 1) {
        view.mouseDown(with: mouse(.leftMouseDown, from, clicks: clicks))
        view.mouseDragged(with: mouse(.leftMouseDragged, to))
        view.mouseUp(with: mouse(.leftMouseUp, to))
        view.drawFrame(force: true)
    }
}

@MainActor
@Suite(.serialized) struct LiveTests {
    @Test func dragSelectsAndCopiesRealText() async throws {
        let live = try LiveHarness("printf 'hello world\\r\\nsecond\\r\\n'")
        #expect(await live.wait(for: "second"))
        live.drag(from: live.point(row: 0, column: 2, half: 0.25), to: live.point(row: 0, column: 8, half: 0.75))
        let saved = NSPasteboard.general.string(forType: .string)
        defer {
            NSPasteboard.general.clearContents()
            if let saved { NSPasteboard.general.setString(saved, forType: .string) }
        }
        live.view.copy(nil)
        #expect(NSPasteboard.general.string(forType: .string) == "llo wor")
        #expect(live.view.hasSelection, "the spans came back after the frame")
        live.drag(from: live.point(row: 0, column: 7), to: live.point(row: 0, column: 7), clicks: 2)
        live.view.copy(nil)
        #expect(NSPasteboard.general.string(forType: .string) == "world")
        live.drag(from: live.point(row: 1, column: 1), to: live.point(row: 1, column: 1), clicks: 3)
        live.view.copy(nil)
        #expect(NSPasteboard.general.string(forType: .string) == "second\n", "a line selection takes its line break")
    }

    @Test func commandClickOpensARealLink() async throws {
        let live = try LiveHarness("printf 'see https://example.com/a now\\r\\n'")
        #expect(await live.wait(for: "now"))
        var opened: [OpenTarget] = []
        live.view.linkOpener = { opened.append($0) }
        live.view.mouseDown(with: live.mouse(.leftMouseDown, live.point(row: 0, column: 10), flags: .command))
        #expect(opened == [.url("https://example.com/a")])
    }

    @Test func aProgramThatAsksForTheMouseGetsReportsAndShiftSelects() async throws {
        let live = try LiveHarness("printf '\\033[?1000h\\033[?1006hready\\r\\n'")
        #expect(await live.wait(for: "ready"))
        live.view.mouseDown(with: live.mouse(.leftMouseDown, live.point(row: 3, column: 2)))
        live.view.mouseUp(with: live.mouse(.leftMouseUp, live.point(row: 3, column: 2)))
        #expect(await live.wait(for: "^[[<0;3;4M"), "the press, encoded by the engine for SGR reporting")
        #expect(await live.wait(for: "^[[<0;3;4m"), "and the release")
    }

    @Test func theKeyboardFollowsTheModesTheProgramSet() async throws {
        let kitty = try LiveHarness("printf '\\033[>1uready\\r\\n'")
        #expect(await kitty.wait(for: "ready"))
        kitty.view.keyDown(
            with: NSEvent.keyEvent(
                with: .keyDown, location: .zero, modifierFlags: .control, timestamp: 0, windowNumber: 0, context: nil, characters: "\u{01}",
                charactersIgnoringModifiers: "a", isARepeat: false, keyCode: 0)!)
        #expect(await kitty.wait(for: "^[[97;5u"), "kitty keyboard: Control+A is CSI 97;5u")
    }

    @Test func aWheelScrollsTheRealHistoryAndTheScrollerFollows() async throws {
        let live = try LiveHarness("i=0; while [ $i -lt 60 ]; do printf 'line %s\\r\\n' $i; i=$((i+1)); done")
        #expect(await live.wait(for: "line 59"))
        let scroller = live.view.scroller
        #expect(scroller.view.isEnabled && scroller.view.doubleValue == 1, "live screen: at the bottom")
        let wheel = try #require(CGEvent(scrollWheelEvent2Source: nil, units: .line, wheelCount: 1, wheel1: 5, wheel2: 0, wheel3: 0))
        live.view.scrollWheel(with: try #require(NSEvent(cgEvent: wheel)))
        live.view.drawFrame(force: true)
        #expect(live.view.renderer.scene.header?.displayOffset == 5)
        #expect(scroller.view.doubleValue < 1 && !scroller.view.isHidden, "an overlay thumb shows while scrolling")
    }

    @Test func pasteIsBracketedOnlyWhenAsked() async throws {
        let live = try LiveHarness("printf '\\033[?2004hready\\r\\n'")
        #expect(await live.wait(for: "ready"))
        live.view.source.paste("one\ntwo")
        #expect(await live.wait(for: "^[[200~one"))
    }
}
#endif

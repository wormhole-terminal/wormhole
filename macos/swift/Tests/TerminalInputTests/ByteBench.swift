#if WITH_UNIFFI
import AppKit
import Testing
import TerminalCore
import UniFFISource

@testable import TerminalMetal

/// The bytes a program receives for what a person does to the view (T05, T07): the view's own event handlers on the real
/// engine and a real PTY, with `perl` as the program, which puts the terminal in raw mode and prints each byte it reads
/// in hexadecimal, so nothing the tty does (echo, the caret form of ESC) stands between the bytes and the check. A program
/// switches the modes on itself, before it reads, by printing the sequences. The events are `NSEvent`s AppKit makes for the view
/// (as `LiveTests` makes them); the window server's routing of a click or a key to the view is the end-to-end suite's.
@MainActor
final class ByteBench {
    let live: LiveHarness
    private var consumed = 0

    /// `modes`: the escape sequences the program writes once it reads raw (`\033[?1000h`), in perl's notation, before it says `go`; the
    /// terminal has taken them by the time `go` is on the screen, and what the engine answers to a query is read like any input.
    init(_ modes: String, cols: Int = 120, rows: Int = 20) throws {
        let reader = #"$|=1; system("stty raw -echo"); print "\#(modes)"; print "go\r\n"; while (sysread(STDIN,$b,1)) { printf "%02x ", ord($b) }"#
        live = try LiveHarness("exec /usr/bin/perl -e '\(reader)'", cols: cols, rows: rows)
    }

    /// The bytes the program printed after it started reading, as `"1b 5b 4d "`.
    private func printed() -> String {
        let rows = (0..<live.view.renderer.gridSize.rows).map { live.view.renderer.text(ofRow: $0) }
        guard let go = rows.firstIndex(where: { $0.trimmingCharacters(in: .whitespaces) == "go" }) else { return "" }
        return rows[(go + 1)...].joined().trimmingCharacters(in: .whitespaces) + " "
    }

    /// Draws frames until `done` holds (the frames pick up what the program wrote); false when it never does.
    func draw(until done: () -> Bool) async -> Bool {
        for _ in 0..<250 {
            live.view.drawFrame(force: true)
            if done() { return true }
            await withCheckedContinuation { next in DispatchQueue.main.asyncAfter(deadline: .now() + 0.02) { next.resume() } }
        }
        return false
    }

    /// Waits until the program reads (it has said `go`).
    func start() async -> Bool { await live.wait(for: "go") }

    static func hex(_ bytes: [UInt8]) -> String { bytes.map { String(format: "%02x ", $0) }.joined() }
    static func hex(_ text: String) -> String { hex(Array(text.utf8)) }

    /// What the program has read since the last call. A paste of `z` after the events is the barrier: a paste is no key, so no mode
    /// of the keyboard changes it, and when its byte has come everything before it has, so the text is whole.
    func read() async -> String {
        live.view.source.paste("z")
        _ = await draw { printed().hasSuffix("7a ") && printed().count > consumed }
        let all = printed()
        let new = String(all.dropFirst(consumed).dropLast(3))
        consumed = all.count
        return new
    }

    /// What the program has read since the last call is exactly `wanted` (`hex` makes it); a failure shows both.
    func expect(_ wanted: String, _ message: String = "", sourceLocation: SourceLocation = #_sourceLocation) async {
        let got = await read()
        #expect(got == wanted, Comment(rawValue: "\(message): read \(got.isEmpty ? "nothing" : got), wanted \(wanted.isEmpty ? "nothing" : wanted)"), sourceLocation: sourceLocation)
    }

    func down(_ type: NSEvent.EventType, row: Int, column: Int, half: CGFloat = 0.5, flags: NSEvent.ModifierFlags = [], clicks: Int = 1) -> NSEvent {
        live.mouse(type, live.point(row: row, column: column, half: half), clicks: clicks, flags: flags)
    }

    /// The middle button's events: AppKit's own constructor gives an `otherMouse` event button number 0, so they are made as the
    /// window server makes them, from a `CGEvent` of the centre button (placed as `wheel` places its events).
    private func middle(_ type: NSEvent.EventType, row: Int, column: Int, flags: NSEvent.ModifierFlags) -> NSEvent {
        let inWindow = live.view.convert(live.point(row: row, column: column), to: nil)
        let point = CGPoint(x: inWindow.x, y: (NSScreen.screens.first?.frame.height ?? 0) - inWindow.y)
        let cgType: CGEventType = type == .otherMouseDown ? .otherMouseDown : .otherMouseUp
        let cg = CGEvent(mouseEventSource: nil, mouseType: cgType, mouseCursorPosition: point, mouseButton: .center)!
        cg.flags = CGEventFlags(rawValue: UInt64(flags.rawValue))
        return NSEvent(cgEvent: cg)!
    }

    func press(_ button: PointerButton, row: Int, column: Int, flags: NSEvent.ModifierFlags = []) {
        switch button {
        case .left: live.view.mouseDown(with: down(.leftMouseDown, row: row, column: column, flags: flags))
        case .right: live.view.rightMouseDown(with: down(.rightMouseDown, row: row, column: column, flags: flags))
        case .middle: live.view.otherMouseDown(with: middle(.otherMouseDown, row: row, column: column, flags: flags))
        case .none: break
        }
    }

    func release(_ button: PointerButton, row: Int, column: Int, flags: NSEvent.ModifierFlags = []) {
        switch button {
        case .left: live.view.mouseUp(with: down(.leftMouseUp, row: row, column: column, flags: flags))
        case .right: live.view.rightMouseUp(with: down(.rightMouseUp, row: row, column: column, flags: flags))
        case .middle: live.view.otherMouseUp(with: middle(.otherMouseUp, row: row, column: column, flags: flags))
        case .none: break
        }
    }

    /// A key as the keyboard sends it to the view: `characters` as the layout types them with the modifiers, `base` without.
    func key(_ characters: String, base: String? = nil, code: UInt16, flags: NSEvent.ModifierFlags = []) {
        let event = NSEvent.keyEvent(
            with: .keyDown, location: .zero, modifierFlags: flags, timestamp: 0, windowNumber: 0, context: nil, characters: characters,
            charactersIgnoringModifiers: base ?? characters, isARepeat: false, keyCode: code)!
        live.view.keyDown(with: event)
    }

    func click(_ button: PointerButton = .left, row: Int, column: Int, flags: NSEvent.ModifierFlags = []) {
        press(button, row: row, column: column, flags: flags)
        release(button, row: row, column: column, flags: flags)
    }

    func drag(to row: Int, _ column: Int) { live.view.mouseDragged(with: down(.leftMouseDragged, row: row, column: column)) }
    func move(to row: Int, _ column: Int, flags: NSEvent.ModifierFlags = []) { live.view.mouseMoved(with: down(.mouseMoved, row: row, column: column, flags: flags)) }

    /// The wheel by `lines` (positive: up) over a cell, as a mouse wheel's line steps. An event made from a `CGEvent` has no
    /// window: its location is the screen's (Quartz counts from the top left of the main display) and the view reads it as the
    /// window's, so the place is given in the screen's terms and checked to come out as the window point it is meant to be.
    func wheel(_ lines: Int, row: Int, column: Int, flags: NSEvent.ModifierFlags = []) throws {
        let inWindow = live.view.convert(live.point(row: row, column: column), to: nil)
        let cg = try #require(CGEvent(scrollWheelEvent2Source: nil, units: .line, wheelCount: 1, wheel1: Int32(lines), wheel2: 0, wheel3: 0))
        cg.location = CGPoint(x: inWindow.x, y: (NSScreen.screens.first?.frame.height ?? 0) - inWindow.y)
        cg.flags = CGEventFlags(rawValue: UInt64(flags.rawValue))
        let event = try #require(NSEvent(cgEvent: cg))
        #expect(abs(event.locationInWindow.x - inWindow.x) < 1 && abs(event.locationInWindow.y - inWindow.y) < 1, "the wheel is at the cell: \(event.locationInWindow) for \(inWindow)")
        live.view.scrollWheel(with: event)
    }
}
#endif

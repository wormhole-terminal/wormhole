// The real `FrameSource`: a `Terminal` of `wormhole-ffi` (the Rust engine, PTY and program) behind the same
// protocol the mock implements. The contract is `macos/FFI-CONTRACT.md`; where this file does something the
// contract leaves open, it says so.

import Foundation
import Synchronization
import TerminalCore
import WormholeBindings

public final class UniFFIFrameSource: FrameSource {
    public enum SourceError: Error { case wrongFrameFormat(rust: UInt32, swift: UInt32) }

    private let program: String
    private let arguments: [String]
    private let terminal = Mutex<Terminal?>(nil)
    /// The wake of a terminal that was started before its view: `start` hands the view's wake to it.
    private let relay: TerminalRelay?
    /// The size the view last asked for, for a terminal that comes after it (`fill`).
    private let fitted = Mutex<(size: GridSize, wake: (@Sendable () -> Void)?)?>(nil)
    private let responsiveness = Mutex<(@Sendable () -> Void)?>(nil)
    private let windowOp = Mutex<(@Sendable (WindowOpRecord) -> Void)?>(nil)
    /// The colours the view last drew with, for a terminal that comes after it (`fill`, `start`).
    private let engineColors = Mutex<[UInt32]?>(nil)

    /// Called, as the view drains events, when the session host stopped answering or answers again.
    public func onHostResponsiveness(_ then: @escaping @Sendable () -> Void) { responsiveness.withLock { $0 = then } }

    /// Called, as the view drains events, when a program asks the window to move, resize, minimise or the like and
    /// `allow_window_ops` lets it (the engine already took the size it asked for).
    public func onWindowOp(_ then: @escaping @Sendable (WindowOpRecord) -> Void) { windowOp.withLock { $0 = then } }

    /// Where the window is, for the reports a program asks for (XTWINOPS 11, 13, 14, 15, 19).
    public func setWindow(_ geometry: WindowGeometryRecord) { current?.setWindow(geometry: geometry) }

    /// Back to the size the terminal has, once the window has followed (or could not follow) a program's resize.
    public func syncSize() { current?.syncSize() }

    public init(program: String, arguments: [String] = []) {
        self.program = program
        self.arguments = arguments
        relay = nil
    }

    /// A source over a terminal that already runs (the workspace started it, `WorkspaceBridge`): `start` only fits it to
    /// the view and gives the view's wake to its relay.
    public init(adopting terminal: Terminal, relay: TerminalRelay) {
        program = ""
        arguments = []
        self.relay = relay
        self.terminal.withLock { $0 = terminal }
    }

    /// A source for a terminal whose program is still being started in, or attached to, a session host (off the main
    /// thread): it draws nothing until `fill` gives it the terminal.
    public init(waiting relay: TerminalRelay) {
        program = ""
        arguments = []
        self.relay = relay
    }

    /// The terminal is ready: fit it to the view's size and wake the view to draw it.
    public func fill(_ terminal: Terminal) {
        self.terminal.withLock { $0 = terminal }
        if let colors = engineColors.withLock({ $0 }) { terminal.setColors(slots: colors) }
        guard let fit = fitted.withLock({ $0 }) else { return }
        terminal.resize(cols: UInt16(fit.size.cols), rows: UInt16(fit.size.rows), cellWidth: fit.size.cellWidth, cellHeight: fit.size.cellHeight)
        fit.wake?()
    }

    /// The Rust side's own time in the frames so far (snapshot and pack), for the measuring mode.
    public var rustFrameTiming: (frames: UInt64, snapshotNs: UInt64, packNs: UInt64, lastBytes: UInt64)? {
        terminal.withLock { $0 }.map {
            let timing = $0.frameTiming()
            return (timing.frames, timing.totalSnapshotNs, timing.totalPackNs, timing.lastBytes)
        }
    }

    public func start(size: GridSize, wake: @escaping @Sendable () -> Void) throws {
        if let relay {
            relay.attach(wake)
            fitted.withLock { $0 = (size, wake) }
            current?.resize(cols: UInt16(size.cols), rows: UInt16(size.rows), cellWidth: size.cellWidth, cellHeight: size.cellHeight)
            return
        }
        // The layouts must agree before one byte of a frame is read.
        guard frameFormatVersion() == FrameLayout.version else {
            throw SourceError.wrongFrameFormat(rust: frameFormatVersion(), swift: FrameLayout.version)
        }
        let created = try Terminal.start(
            program: program, args: arguments, cols: UInt16(size.cols), rows: UInt16(size.rows),
            cellWidth: size.cellWidth, cellHeight: size.cellHeight, listener: WakeListener(wake))
        terminal.withLock { $0 = created }
        if let colors = engineColors.withLock({ $0 }) { created.setColors(slots: colors) }
    }

    /// A new small object per call: the view takes it once, after the source started.
    public var interaction: (any TerminalInteraction)? { current.map(UniFFIInteraction.init) }

    public var scrollback: (any TerminalScrollback)? { current.map(UniFFIScrollback.init) }

    public var probes: (any TerminalProbes)? { current.map(UniFFIProbes.init) }

    public func stop() {
        terminal.withLock { current in
            current?.close()
            current = nil
        }
    }

    /// Let go of the terminal without ending it: the terminal is moving to another window, whose own source takes it. This
    /// source (and the view over it) is finished; stopping it afterwards does nothing.
    public func detach() { terminal.withLock { $0 = nil } }

    private var current: Terminal? { terminal.withLock { $0 } }

    /// A still screen's frame is a length of 0 and nothing else: no bytes cross, nothing is allocated.
    public func frame() -> Data {
        guard let current, current.prepareFrame() > 0 else { return Data() }
        return current.takeFrame()
    }
    public func fullFrame() -> Data { current?.fullFrame() ?? Data() }

    public func drainEvents() -> [TerminalEvent] {
        var events: [TerminalEvent] = []
        for event in current?.drainEvents() ?? [] {
            switch event {
            case .output: events.append(.output)
            case .title(let title): events.append(.title(title))
            case .resetTitle: events.append(.resetTitle)
            case .bell: events.append(.bell)
            case .exited(let code): events.append(.exited(code: code))
            case .clipboardWrite(let text): events.append(.clipboardWrite(text))
            case .clipboardRead: events.append(.clipboardRead)
            case .clipboardAsk: events.append(.clipboardAsk)
            case .clipboardAskClosed: events.append(.clipboardAskClosed)
            case .controlTyped: events.append(.controlTyped)
            // The bar over the pane says it; the view has nothing to draw.
            case .hostResponsiveness: responsiveness.withLock { $0 }?()
            case .windowOp(let op): windowOp.withLock { $0 }?(op)
            @unknown default: break
            }
        }
        return events
    }

    public func write(_ bytes: Data) { current?.write(bytes: bytes) }

    /// Told to the terminal when they are not the ones it was last told (the engine would report a scheme again).
    public func setEngineColors(_ slots: [UInt32]) {
        let moved = engineColors.withLock { known -> Bool in
            if known == slots { return false }
            known = slots
            return true
        }
        if moved { current?.setColors(slots: slots) }
    }

    public func setClipboardRules(read: TerminalCore.ClipboardReadRule, writeAllowed: Bool) {
        let rule: WormholeBindings.ClipboardReadRule = switch read {
        case .ask: .ask
        case .allow: .allow
        case .deny: .deny
        }
        current?.setClipboardRules(read: rule, writeAllowed: writeAllowed)
    }

    public func setOnScreen(_ onScreen: Bool) { current?.setOnScreen(onScreen: onScreen) }
    public func giveClipboard(_ text: String) { current?.clipboardGive(text: text) }
    public func foregroundProgram() -> String? { current?.foregroundProgram() }

    public func answerClipboard(_ choice: TerminalCore.ClipboardChoice, text: String) {
        let answer: WormholeBindings.ClipboardChoice = switch choice {
        case .once: .once
        case .always: .always
        case .deny: .deny
        }
        current?.clipboardAnswer(choice: answer, text: text)
    }

    public var encodesKeys: Bool { true }

    public func sendKey(_ key: KeyInput) -> Bool {
        current?.sendKey(
            press: KeyPress(
                key: key.key, text: key.text, shift: key.shift, alt: key.alt, control: key.control,
                platform: key.platform, optionAsMeta: key.optionAsMeta)) ?? false
    }

    public func resize(_ size: GridSize) {
        fitted.withLock { $0?.size = size }
        current?.resize(cols: UInt16(size.cols), rows: UInt16(size.rows), cellWidth: size.cellWidth, cellHeight: size.cellHeight)
    }

    public func scroll(lines: Int) { current?.scroll(lines: Int32(clamping: lines)) }

    public func scrollRows(_ rows: Int) { current?.scrollRows(rows: Int32(clamping: rows)) }

    public func paste(_ text: String) { current?.paste(text: text) }

    public func plainText(history: Bool) -> String? { current?.contents(scope: history ? .everything : .screen) }

    public func grabPrompt() -> String? { current?.grabPrompt() }
    public var takesPasteAsText: Bool { current?.takesPasteAsText() ?? false }

    /// Rust's record has the same name as TerminalCore's value, hence the qualified one.
    public func inlineImage(key: UInt32) -> TerminalCore.InlineImage? {
        current?.inlineImage(key: key).map {
            TerminalCore.InlineImage(width: Int($0.width), height: Int($0.height), rgba: $0.rgba)
        }
    }

    /// The visible screen as plain text.
    public func screenText() -> String { current?.screenText() ?? "" }
}

/// What Rust holds: it calls `wake()` on its event thread, at most once between two drains. It only forwards
/// to the closure (the view's relay), which sets an atomic and hops to the main queue.
private final class WakeListener: TerminalListener {
    private let onWake: @Sendable () -> Void
    init(_ onWake: @escaping @Sendable () -> Void) { self.onWake = onWake }
    func wake() { onWake() }
}

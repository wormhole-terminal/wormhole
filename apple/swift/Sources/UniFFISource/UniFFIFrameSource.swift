// The real `FrameSource`: a `Terminal` of `wormhole-ffi` (the Rust engine, PTY and program) behind the same
// protocol the mock implements. The contract is `apple/FFI-CONTRACT.md`; where this file does something the
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

    /// The Rust side's own time in the frames so far (snapshot and pack), for the measuring mode.
    public var rustFrameTiming: (frames: UInt64, snapshotNs: UInt64, packNs: UInt64, lastBytes: UInt64)? {
        terminal.withLock { $0 }.map {
            let timing = $0.frameTiming()
            return (timing.frames, timing.totalSnapshotNs, timing.totalPackNs, timing.lastBytes)
        }
    }

    public func start(size: GridSize, wake: @escaping @Sendable () -> Void) throws {
        if let relay, let adopted = current {
            relay.attach(wake)
            adopted.resize(cols: UInt16(size.cols), rows: UInt16(size.rows), cellWidth: size.cellWidth, cellHeight: size.cellHeight)
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
    }

    /// A new small object per call: the view takes it once, after the source started.
    public var interaction: (any TerminalInteraction)? { current.map(UniFFIInteraction.init) }

    public var scrollback: (any TerminalScrollback)? { current.map(UniFFIScrollback.init) }

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

    public func frame() -> Data { current?.frame() ?? Data() }
    public func fullFrame() -> Data { current?.fullFrame() ?? Data() }

    public func drainEvents() -> [TerminalEvent] {
        (current?.drainEvents() ?? []).compactMap { event in
            switch event {
            case .output: .output
            case .title(let title): .title(title)
            case .resetTitle: .resetTitle
            case .bell: .bell
            case .exited(let code): .exited(code: code)
            case .clipboardWrite(let text): .clipboardWrite(text)
            case .clipboardRead: .clipboardRead
            case .clipboardAsk: .clipboardAsk
            case .clipboardAskClosed: .clipboardAskClosed
            @unknown default: nil
            }
        }
    }

    public func write(_ bytes: Data) { current?.write(bytes: bytes) }

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
        current?.resize(cols: UInt16(size.cols), rows: UInt16(size.rows), cellWidth: size.cellWidth, cellHeight: size.cellHeight)
    }

    public func scroll(lines: Int) { current?.scroll(lines: Int32(clamping: lines)) }

    public func paste(_ text: String) { current?.paste(text: text) }

    public func plainText(history: Bool) -> String? { current?.contents(scope: history ? .everything : .screen) }

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

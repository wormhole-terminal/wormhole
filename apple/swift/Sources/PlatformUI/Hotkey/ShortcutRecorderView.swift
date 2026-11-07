// The shortcut recorder as an `NSView`: click it, press the keys. Esc stops and keeps the shortcut there was, ⌫
// clears it, a key without a modifier waits (other than an F-key), and a combination somebody has already (the
// app, macOS, the shell) is refused with what has it, instead of being taken. What a press means is
// `ShortcutRecording` (core's `shortcut_record`); this only turns key events into it and draws the pill.

import AppKit
import PlatformModel

@MainActor
public final class ShortcutRecorderView: NSView {
    public var shortcut: KeySpec? { didSet { needsDisplay = true; updateAccessibility() } }
    /// The action the field sets the key of, whose own key is no conflict (the jump key is Go to Waiting's).
    public var ownAction: ActionID?
    public var keymap: Keymap = .bundled
    public var style: ShortcutRecorderStyle? { didSet { needsDisplay = true } }
    public var onChange: (KeySpec?) -> Void = { _ in }
    /// What stands in the way of the last key pressed, or nil: the owner shows it under the field.
    public var onMessage: (String?) -> Void = { _ in }
    public private(set) var isRecording = false { didSet { needsDisplay = true; updateAccessibility() } }
    private var held: KeyMods = []

    public override var acceptsFirstResponder: Bool { true }
    public override var intrinsicContentSize: NSSize { NSSize(width: 160, height: 24) }

    public override init(frame: NSRect) {
        super.init(frame: frame)
        setAccessibilityRole(.button)
        setAccessibilityLabel("Shortcut")
        updateAccessibility()
    }

    public required init?(coder: NSCoder) { nil }

    public override func becomeFirstResponder() -> Bool {
        isRecording = true
        held = []
        onMessage(nil)
        return true
    }

    public override func resignFirstResponder() -> Bool {
        isRecording = false
        return true
    }

    public override func mouseDown(with event: NSEvent) { window?.makeFirstResponder(self) }

    /// While recording every key is the recorder's, ⌘ combinations included: the menu bar must not run them.
    public override func performKeyEquivalent(with event: NSEvent) -> Bool {
        guard isRecording, event.type == .keyDown else { return false }
        record(event)
        return true
    }

    public override func keyDown(with event: NSEvent) { isRecording ? record(event) : super.keyDown(with: event) }

    public override func flagsChanged(with event: NSEvent) {
        held = Self.mods(event.modifierFlags)
        needsDisplay = true
    }

    private func record(_ event: NSEvent) {
        guard let key = KeyCodes.name(forCode: event.keyCode) else { return NSSound.beep() }
        switch ShortcutRecording.record(Self.mods(event.modifierFlags), key: key) {
        case .cancel: window?.makeFirstResponder(nil)
        case .clear:
            shortcut = nil
            onChange(nil)
            window?.makeFirstResponder(nil)
        case .needsModifier: onMessage("Add ⌘, ⌃ or ⌥ to the key")
        case .shortcut(let spec):
            if let conflict = ShortcutRecording.conflict(spec, own: ownAction, in: keymap) { return onMessage(conflict) }
            shortcut = spec
            onChange(spec)
            window?.makeFirstResponder(nil)
        }
    }

    static func mods(_ flags: NSEvent.ModifierFlags) -> KeyMods {
        var mods: KeyMods = []
        if flags.contains(.command) { mods.insert(.command) }
        if flags.contains(.option) { mods.insert(.option) }
        if flags.contains(.control) { mods.insert(.control) }
        if flags.contains(.shift) { mods.insert(.shift) }
        return mods
    }

    /// What the pill says: the held modifiers while recording, the shortcut otherwise.
    var label: String {
        if isRecording { return held.isEmpty ? "Type shortcut" : KeySpec(held, "").glyphText }
        return shortcut?.glyphText ?? "None"
    }

    private func updateAccessibility() {
        setAccessibilityValue(isRecording ? "Recording" : shortcut.map { KeyCapSpeech.words($0.glyphText) } ?? "None")
        setAccessibilityHelp("Press to record a new shortcut. Escape cancels, Delete clears.")
    }

    public override func draw(_ dirtyRect: NSRect) {
        guard let style else { return }
        let pill = NSBezierPath(roundedRect: bounds.insetBy(dx: 1, dy: 1), xRadius: style.radius, yRadius: style.radius)
        style.fill.setFill()
        pill.fill()
        if isRecording {
            style.ring.setStroke()
            pill.lineWidth = 1.5
            pill.stroke()
        }
        let color = isRecording || shortcut != nil ? style.text : style.muted
        let text = NSAttributedString(string: label, attributes: [.font: NSFont.systemFont(ofSize: 13), .foregroundColor: color])
        let size = text.size()
        text.draw(at: NSPoint(x: (bounds.width - size.width) / 2, y: (bounds.height - size.height) / 2))
    }
}

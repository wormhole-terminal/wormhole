// The overlays' text field. SwiftUI's `TextField` hands ↑ ↓ Tab and Esc to the focus system, so the palette's keys
// (F-70) would never arrive; an `NSTextField` reports them as editing commands the owner can take first. It also
// lets the placeholder, text and caret take their colours from the theme instead of the system's.

import AppKit
import SwiftUI

/// The keys the palette takes before the field does. `handled` stops the field acting on it.
enum FieldKey: Equatable {
    case up, down, tab(forward: Bool), enter, escape
    /// ⌫ in an empty field, ⌘⌫ and ⌘C with nothing selected in the field (the result's own copy).
    case backspaceWhenEmpty, commandDelete, commandCopy
    /// ⌘↩: the result in a new window.
    case commandEnter
}

struct PaletteField: NSViewRepresentable {
    @Binding var text: String
    let placeholder: String
    let size: CGFloat
    let textColor: Color
    let placeholderColor: Color
    let caretColor: Color
    /// Returns whether the key was used.
    let onKey: (FieldKey) -> Bool

    func makeNSView(context: Context) -> FocusingField {
        let field = FocusingField()
        field.isBordered = false
        field.drawsBackground = false
        field.focusRingType = .none
        field.usesSingleLineMode = true
        field.cell?.isScrollable = true
        field.delegate = context.coordinator
        field.onCommandCopy = { [weak coordinator = context.coordinator] in coordinator?.parent.onKey(.commandCopy) ?? false }
        field.onCommandEnter = { [weak coordinator = context.coordinator] in coordinator?.parent.onKey(.commandEnter) ?? false }
        return field
    }

    func updateNSView(_ field: FocusingField, context: Context) {
        context.coordinator.parent = self
        if field.stringValue != text { field.stringValue = text }
        field.font = .systemFont(ofSize: size)
        field.textColor = NSColor(textColor)
        field.caretColor = NSColor(caretColor)
        field.placeholderAttributedString = NSAttributedString(
            string: placeholder,
            attributes: [.foregroundColor: NSColor(placeholderColor), .font: NSFont.systemFont(ofSize: size)])
        field.setAccessibilityLabel(placeholder)
    }

    func sizeThatFits(_ proposal: ProposedViewSize, nsView: FocusingField, context: Context) -> CGSize? {
        CGSize(width: proposal.width ?? 200, height: nsView.intrinsicContentSize.height)
    }

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    @MainActor final class Coordinator: NSObject, NSTextFieldDelegate {
        var parent: PaletteField
        init(_ parent: PaletteField) { self.parent = parent }

        func controlTextDidChange(_ note: Notification) {
            guard let field = note.object as? NSTextField else { return }
            parent.text = field.stringValue
        }

        func control(_ control: NSControl, textView: NSTextView, doCommandBy command: Selector) -> Bool {
            let key: FieldKey? =
                switch command {
                case #selector(NSResponder.moveUp(_:)): .up
                case #selector(NSResponder.moveDown(_:)): .down
                case #selector(NSResponder.insertTab(_:)): .tab(forward: true)
                case #selector(NSResponder.insertBacktab(_:)): .tab(forward: false)
                case #selector(NSResponder.insertNewline(_:)): .enter
                case #selector(NSResponder.cancelOperation(_:)): .escape
                case #selector(NSResponder.deleteToBeginningOfLine(_:)): .commandDelete
                case #selector(NSResponder.deleteBackward(_:)) where textView.string.isEmpty: .backspaceWhenEmpty
                default: nil
                }
            return key.map(parent.onKey) ?? false
        }
    }
}

/// Takes the keyboard when it joins a window, as the GPUI palette's field does when it opens.
final class FocusingField: NSTextField {
    var caretColor: NSColor = .labelColor
    /// ⌘C with nothing selected in the field: the owner may take it (a setting's key); true when it did.
    var onCommandCopy: () -> Bool = { false }
    /// ⌘↩, which a field would take as a plain return.
    var onCommandEnter: () -> Bool = { false }
    private var monitor: Any?

    /// The two ⌘ keys are taken before the window's key equivalents: the hosting view above does not pass them down to a
    /// field, and the Edit menu's Copy would act on an empty selection.
    private func hearCommandKeys() {
        guard monitor == nil else { return }
        monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard let self, event.window === self.window,
                event.modifierFlags.intersection(.deviceIndependentFlagsMask) == .command
            else { return event }
            let taken =
                (event.keyCode == 36 || event.keyCode == 76) ? self.onCommandEnter()
                : event.charactersIgnoringModifiers == "c" && self.currentEditor()?.selectedRange.length == 0 ? self.onCommandCopy() : false
            return taken ? nil : event
        }
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        if window == nil {
            monitor.map(NSEvent.removeMonitor)
            monitor = nil
        } else {
            hearCommandKeys()
        }
        DispatchQueue.main.async { [weak self] in self?.window?.makeFirstResponder(self) }
    }

    override func becomeFirstResponder() -> Bool {
        let became = super.becomeFirstResponder()
        (currentEditor() as? NSTextView)?.insertionPointColor = caretColor
        return became
    }
}

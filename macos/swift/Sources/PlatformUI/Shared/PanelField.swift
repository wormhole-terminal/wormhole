// The text field of the floating panels (quick panel, folder picker). SwiftUI's `TextField` hands ↑ ↓ ⇥ and Esc
// to the focus system, so a panel's keys would never arrive; an `NSTextField` reports them as editing commands the
// owner can take first, and the system's own field keeps input methods, dictation and the spelling of the keys.
// Text, placeholder and caret take the theme's colours; a ghost completion shows grey after the caret.

import AppKit
import SwiftUI

/// The keys a panel takes before its field does.
enum PanelKey: Equatable {
    case up, down, tab, enter, escape, pageUp, pageDown
    /// ⌘⌫ on the selected row.
    case commandDelete
    /// ← and → with the field's state, which decides whether the panel or the caret takes them.
    case left, right
}

struct PanelField: NSViewRepresentable {
    @Binding var text: String
    var placeholder: String
    var size: CGFloat
    var textColor: Color
    var placeholderColor: Color
    var caretColor: Color
    /// Grey text after the caret.
    var ghost: String? = nil
    var ghostColor: Color = .secondary
    /// Where the caret is and how much is selected, reported on every change: ← and → depend on it.
    var onSelection: (Int, Int) -> Void = { _, _ in }
    /// Returns whether the key was used.
    var onKey: (PanelKey) -> Bool

    func makeNSView(context: Context) -> GhostField {
        let field = GhostField()
        field.isBordered = false
        field.drawsBackground = false
        field.focusRingType = .none
        field.usesSingleLineMode = true
        field.cell?.isScrollable = true
        field.delegate = context.coordinator
        return field
    }

    func updateNSView(_ field: GhostField, context: Context) {
        context.coordinator.parent = self
        if field.stringValue != text { field.stringValue = text }
        field.font = .systemFont(ofSize: size)
        field.textColor = NSColor(textColor)
        field.caretColor = NSColor(caretColor)
        field.ghost = ghost
        field.ghostColor = NSColor(ghostColor)
        field.placeholderAttributedString = NSAttributedString(
            string: placeholder, attributes: [.foregroundColor: NSColor(placeholderColor), .font: NSFont.systemFont(ofSize: size)])
        field.setAccessibilityLabel(placeholder)
    }

    func sizeThatFits(_ proposal: ProposedViewSize, nsView: GhostField, context: Context) -> CGSize? {
        CGSize(width: proposal.width ?? 200, height: nsView.intrinsicContentSize.height)
    }

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    @MainActor final class Coordinator: NSObject, NSTextFieldDelegate {
        var parent: PanelField
        init(_ parent: PanelField) { self.parent = parent }

        func controlTextDidChange(_ note: Notification) {
            guard let field = note.object as? NSTextField else { return }
            parent.text = field.stringValue
            reportSelection(field)
        }

        func controlTextDidEndEditing(_ note: Notification) {}

        private func reportSelection(_ field: NSTextField) {
            guard let range = (field.currentEditor() as? NSTextView)?.selectedRange() else { return }
            parent.onSelection(range.location, range.length)
        }

        func control(_ control: NSControl, textView: NSTextView, doCommandBy command: Selector) -> Bool {
            reportSelection(control as? NSTextField ?? NSTextField())
            let key: PanelKey? =
                switch command {
                case #selector(NSResponder.moveUp(_:)): .up
                case #selector(NSResponder.moveDown(_:)): .down
                case #selector(NSResponder.pageUp(_:)), #selector(NSResponder.scrollPageUp(_:)): .pageUp
                case #selector(NSResponder.pageDown(_:)), #selector(NSResponder.scrollPageDown(_:)): .pageDown
                case #selector(NSResponder.moveRight(_:)): .right
                case #selector(NSResponder.moveLeft(_:)): .left
                case #selector(NSResponder.insertTab(_:)): .tab
                case #selector(NSResponder.insertNewline(_:)): .enter
                case #selector(NSResponder.cancelOperation(_:)): .escape
                case #selector(NSResponder.deleteToBeginningOfLine(_:)): .commandDelete
                default: nil
                }
            return key.map(parent.onKey) ?? false
        }
    }
}

/// Takes the keyboard when it joins a window, and draws its ghost completion after the caret.
final class GhostField: NSTextField {
    var caretColor: NSColor = .labelColor
    var ghost: String? { didSet { needsDisplay = true } }
    var ghostColor: NSColor = .secondaryLabelColor

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        DispatchQueue.main.async { [weak self] in self?.window?.makeFirstResponder(self) }
    }

    override func becomeFirstResponder() -> Bool {
        let became = super.becomeFirstResponder()
        (currentEditor() as? NSTextView)?.insertionPointColor = caretColor
        return became
    }

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        guard let ghost, let font, !stringValue.isEmpty else { return }
        let typed = NSAttributedString(string: stringValue, attributes: [.font: font]).size().width
        let line = NSAttributedString(string: ghost, attributes: [.font: font, .foregroundColor: ghostColor])
        let height = line.size().height
        line.draw(at: NSPoint(x: typed + 2, y: (bounds.height - height) / 2))
    }
}

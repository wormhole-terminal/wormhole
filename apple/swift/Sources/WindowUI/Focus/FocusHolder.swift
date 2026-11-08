// The one view that holds the keyboard for the areas that are rows of buttons, tabs and items (the title row, the
// terminal's tab strip, the panel's tabs, the status bar): a real first responder, so the terminal gives the keyboard
// up (hollow cursor, focus out to a program that asked) and VoiceOver has an element to follow. Which item the keys are
// on is `WindowFocus`'s; this says it aloud and hands every key to it.

import AppKit
import SwiftUI

struct FocusHolder: NSViewRepresentable {
    let focus: WindowFocus

    func makeNSView(context: Context) -> FocusHolderView {
        let view = FocusHolderView(focus: focus)
        focus.holder = view
        return view
    }

    func updateNSView(_ view: FocusHolderView, context: Context) {}

    static func dismantleNSView(_ view: FocusHolderView, coordinator: ()) { view.focus?.detach() }
}

final class FocusHolderView: NSView {
    weak var focus: WindowFocus?

    init(focus: WindowFocus) {
        self.focus = focus
        super.init(frame: .zero)
        setAccessibilityElement(true)
        setAccessibilityRole(.group)
    }

    @available(*, unavailable) required init?(coder: NSCoder) { fatalError("not decoded") }

    override var acceptsFirstResponder: Bool { true }
    override var focusRingType: NSFocusRingType { get { .none } set {} }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        if window != nil { focus?.attach(to: self) }
    }

    override func becomeFirstResponder() -> Bool {
        focus?.holderChanged(holding: true)
        return true
    }

    override func resignFirstResponder() -> Bool {
        focus?.holderChanged(holding: false)
        return true
    }

    override func keyDown(with event: NSEvent) {
        if focus?.holderKey(event) != true { super.keyDown(with: event) }
    }

    /// What VoiceOver says for the place the keys are on, and that it moved.
    func speak(_ words: String) {
        setAccessibilityLabel(words)
        NSAccessibility.post(element: self, notification: .focusedUIElementChanged)
    }
}

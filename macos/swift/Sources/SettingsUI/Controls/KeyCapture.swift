// Captures the next key press while a recorder is recording: a first-responder view that reads `keyDown` and
// also takes ⌘-keys (`performKeyEquivalent`), which would otherwise run the menu.

import AppKit
import DesignSystem
import SwiftUI

enum RecordedKey: Equatable {
    case spec(String)
    case cancel
    case clear
}

struct KeyCapture: NSViewRepresentable {
    let onKey: (RecordedKey) -> Void

    func makeNSView(context: Context) -> CaptureView {
        let view = CaptureView()
        view.onKey = onKey
        DispatchQueue.main.async { view.window?.makeFirstResponder(view) }
        return view
    }

    func updateNSView(_ view: CaptureView, context: Context) { view.onKey = onKey }

    /// A recorder (`RecordsKeys`): the app's key monitors leave its presses alone.
    final class CaptureView: NSView, RecordsKeys {
        var onKey: (RecordedKey) -> Void = { _ in }
        override var acceptsFirstResponder: Bool { true }

        override func keyDown(with event: NSEvent) { handle(event) }

        override func performKeyEquivalent(with event: NSEvent) -> Bool {
            handle(event)
            return true
        }

        private func handle(_ event: NSEvent) {
            let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
            switch event.keyCode {
            case 53 where flags.isEmpty: onKey(.cancel)
            case 51 where flags.isEmpty: onKey(.clear)
            default:
                if let spec = KeySpecText.spec(keyCode: event.keyCode, characters: event.charactersIgnoringModifiers, modifiers: flags) {
                    onKey(.spec(spec))
                }
            }
        }
    }
}

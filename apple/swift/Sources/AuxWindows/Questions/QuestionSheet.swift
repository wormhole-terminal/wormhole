// A question as the system's own alert, a sheet on the window (`dialog.rs`): macOS draws it, so it looks and
// behaves as every Mac alert does: Return presses the first button, Esc the last (Cancel), VoiceOver reads it as
// an alert, and a checkbox, where there is one, is the alert's own. A window that can't take a sheet (none is
// open) gets it as a modal alert.

import AppKit
import AuxModel

@MainActor
public final class QuestionSheet {
    private let alert = NSAlert()
    /// The alert's buttons as the person reads them, first the default (for tests: the alert is the system's window).
    var buttonTitles: [String] { alert.buttons.map(\.title) }
    private weak var window: NSWindow?
    private let question: Question

    public init(_ question: Question) {
        self.question = question
        alert.messageText = question.title
        alert.informativeText = question.detail
        alert.alertStyle = question.style == .warning ? .warning : .informational
        alert.addButton(withTitle: question.confirm)
        for title in question.alsoButtons { alert.addButton(withTitle: title) }
        let cancel = alert.addButton(withTitle: question.cancel)
        cancel.keyEquivalent = "\u{1b}"
        if let box = question.checkbox {
            alert.showsSuppressionButton = true
            alert.suppressionButton?.title = box.label
            alert.suppressionButton?.state = box.checked ? .on : .off
        }
    }

    /// Asks, on `window` as a sheet. The answer is what the buttons say; the box is read with it.
    public func ask(in window: NSWindow?) async -> Answer {
        self.window = window
        let response: NSApplication.ModalResponse
        if let window {
            response = await alert.beginSheetModal(for: window)
        } else {
            response = alert.runModal()
        }
        return answer(for: response)
    }

    /// Takes the sheet away without an answer (the window it was about closed under it).
    public func withdraw() {
        guard let window, window.attachedSheet === alert.window else { return }
        window.endSheet(alert.window, returnCode: .cancel)
    }

    private func answer(for response: NSApplication.ModalResponse) -> Answer {
        let place = response.rawValue - NSApplication.ModalResponse.alertFirstButtonReturn.rawValue
        switch place {
        case 0: return .confirm(boxChecked: alert.suppressionButton?.state == .on)
        case 1...question.alsoButtons.count: return .also(place - 1)
        default: return .cancel
        }
    }
}

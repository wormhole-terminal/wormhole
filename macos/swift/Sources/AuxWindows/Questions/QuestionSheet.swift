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
        if let pending = PendingQuestions.current { return await held(by: pending) }
        let response: NSApplication.ModalResponse
        if let window {
            response = await alert.beginSheetModal(for: window)
        } else {
            response = alert.runModal()
        }
        return answer(for: response)
    }

    /// A test instance holds the question for a driver instead of presenting it.
    private func held(by pending: PendingQuestions) async -> Answer {
        let buttons = [question.confirm] + question.alsoButtons + [question.cancel]
        let place = await pending.hold(
            title: question.title, detail: question.detail, buttons: buttons, confirm: 0, cancel: buttons.count - 1)
        return answer(at: place)
    }

    /// Takes the sheet away without an answer (the window it was about closed under it).
    public func withdraw() {
        guard let window, window.attachedSheet === alert.window else { return }
        window.endSheet(alert.window, returnCode: .cancel)
    }

    private func answer(for response: NSApplication.ModalResponse) -> Answer {
        answer(at: response.rawValue - NSApplication.ModalResponse.alertFirstButtonReturn.rawValue)
    }

    /// The button at `place`: the first confirms, the extra ones go ahead another way, the last backs out. (A range `1...0`
    /// traps, so a question with no extra button, whose Cancel is place 1, must not be matched against one.)
    func answer(at place: Int) -> Answer {
        if place == 0 { return .confirm(boxChecked: alert.suppressionButton?.state == .on) }
        return place >= 1 && place <= question.alsoButtons.count ? .also(place - 1) : .cancel
    }
}

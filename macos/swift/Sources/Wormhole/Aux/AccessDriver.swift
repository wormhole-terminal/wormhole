// Puts the access backend's questions on a window of the profile they are about as sheets, one at a time, as they are queued
// or settled elsewhere (the core's queue wakes it): first a mode widened outside the app, then the programs'. A sheet closed
// without an answer counts as no for now, not remembered; one nobody answers in a minute goes by itself (the core times it
// out and wakes this, which finds the question gone).

import AppKit
import AuxModel
import AuxWindows

@MainActor
final class AccessDriver {
    private let backend: any AccessBackend
    /// The window to ask on: one of this profile's, nil when none is open (the question then counts as not answered).
    private let window: @MainActor () -> NSWindow?
    private var showing: (sheet: AccessSheet, stillWaiting: () -> Bool)?
    private var watching: Task<Void, Never>?
    /// The question that is up: a program's, or the one about a mode widened outside the app (`AccessDriver+Scripted`).
    var asked: AccessQuestion?
    var askedMode: ModeQuestion?
    var answers: any AccessBackend { backend }

    init(backend: any AccessBackend, window: @escaping @MainActor () -> NSWindow?) {
        self.backend = backend
        self.window = window
    }

    func start() {
        let changes = backend.changes()
        watching = Task { [weak self] in
            for await _ in changes { self?.present() }
        }
        present()
    }

    private func present() {
        if let showing, !showing.stillWaiting() { showing.sheet.withdraw() }
        guard showing == nil else { return }
        if let change = backend.nextModeQuestion() {
            askedMode = change
            ask(AccessSheet(AccessDisplay.modeChanged(previous: change.previous, requested: change.requested)), waiting: { [backend] in backend.isWaitingMode(change.token) }) {
                [backend] button in backend.answerMode(change.token, keep: button == .keep)
            }
        } else if let question = backend.next() {
            // A test instance's script answers at once, with no sheet (`agent-script`).
            if let wish = Self.script { return backend.answer(question.id, Self.offered(AccessDisplay.of(question.request).buttons, wish: wish)) }
            asked = question
            ask(AccessSheet(question.request), waiting: { [backend] in backend.isWaiting(question.id) }) { [backend] button in
                if let button { backend.answer(question.id, button) } else { backend.withdraw(question.id) }
            }
        }
    }

    private func ask(_ sheet: AccessSheet, waiting: @escaping () -> Bool, settle: @escaping (AccessButton?) -> Void) {
        showing = (sheet, waiting)
        Task {
            // A profile with no window open cannot be asked: not answered.
            let button = window() == nil ? nil : await sheet.ask(in: window())
            settle(button)
            showing = nil
            (asked, askedMode) = (nil, nil)
            present()
        }
    }
}

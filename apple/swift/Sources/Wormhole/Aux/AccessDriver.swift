// Puts the access backend's questions on the window as sheets, one at a time, as they are queued or settled elsewhere
// (the core's queue wakes it); a sheet closed without an answer counts as no for now, not remembered.

import AppKit
import AuxModel
import AuxWindows

@MainActor
final class AccessDriver {
    private let backend: any AccessBackend
    private let window: @MainActor () -> NSWindow?
    private var showing = false
    private var watching: Task<Void, Never>?

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
        guard !showing, let question = backend.next() else { return }
        showing = true
        let sheet = AccessSheet(question.request)
        Task {
            if let button = await sheet.ask(in: window()) { backend.answer(question.id, button) } else { backend.withdraw(question.id) }
            showing = false
            present()
        }
    }
}

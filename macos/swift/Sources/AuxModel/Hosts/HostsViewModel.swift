// The Session Hosts window's state (`hosts_window/mod.rs`): the report as the backend gathers it, looked at again when
// the hosts folder changes and on a slow timer while the window is open (a watch can fail or sit on a volume that
// sends none), what was just copied, and End and Forget. The window draws again only when something shows
// differently.

import Foundation
import Observation

@MainActor @Observable
public final class HostsViewModel {
    /// What was copied, for the button to say "Copied" for a moment.
    public enum Copied: Equatable, Sendable { case all, host(String) }

    public private(set) var report: HostsReport?
    public private(set) var copied: Copied?
    /// The hosts whose cards are opened, by id: a card starts folded.
    public private(set) var open: Set<String> = []
    /// The list is to scroll to this offset (points from the top): the control folder's `hosts-scroll`; `serial` makes a repeat count.
    public private(set) var scrollRequest: (offset: Double, serial: Int)?
    @ObservationIgnored public let backend: any HostsBackend
    @ObservationIgnored private let write: (String) -> Void
    @ObservationIgnored private let slowLook: Duration
    @ObservationIgnored private var watch: Task<Void, Never>?
    @ObservationIgnored private var poll: Task<Void, Never>?

    /// How long "Copied" stays up.
    public static let copiedFor = Duration.milliseconds(1500)

    /// `write` puts text on the pasteboard; `slowLook` is the poll that backs the folder watch up.
    public init(backend: any HostsBackend, slowLook: Duration = .seconds(3), write: @escaping (String) -> Void) {
        (self.backend, self.slowLook, self.write) = (backend, slowLook, write)
    }

    public func start() {
        stop()
        let changes = backend.changes()
        watch = Task { [weak self] in for await _ in changes { await self?.refresh() } }
        poll = Task { [weak self, slowLook] in
            while !Task.isCancelled {
                await self?.refresh()
                // one-shot: the next look of the slow fallback behind the folder watch (a watch can drop events), armed after each look
                try? await Task.sleep(for: slowLook)
            }
        }
    }

    public func stop() {
        watch?.cancel()
        poll?.cancel()
    }

    public func refresh() async {
        let next = await backend.report()
        if next != report { report = next }
    }

    public func scroll(to offset: Double) { scrollRequest = (offset, (scrollRequest?.serial ?? 0) + 1) }

    /// Opens a host's card, or folds it.
    public func toggle(_ id: String) {
        if open.contains(id) { open.remove(id) } else { open.insert(id) }
    }

    public func copy(_ what: Copied) {
        guard let report else { return }
        switch what {
        case .all: write(report.text)
        case .host(let id):
            guard let card = report.cards.first(where: { $0.id == id }) else { return }
            write(card.text)
        }
        copied = what
        Task { [weak self] in
            // one-shot: "Copied" goes away after copiedFor
            try? await Task.sleep(for: Self.copiedFor)
            if self?.copied == what { self?.copied = nil }
        }
    }

    /// The question before ending a host: it hangs up the program in it.
    public func endQuestion(_ id: String) -> Question? {
        report?.cards.first { $0.id == id }.map { CloseQuestions.endHost(id: id, attachedTitle: $0.attachedTitle) }
    }

    public func end(_ id: String) { Task { await backend.end(id); await refresh() } }
    public func forget(_ id: String) { Task { await backend.forget(id); await refresh() } }
}

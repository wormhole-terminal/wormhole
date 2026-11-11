// The Usage tab's state: what each agent last reported of its plan's limits. The reports are the account's, one value
// for every window; the words that change with time ("resets 34m") are made by the view from `now`.

import Observation
import PanelModel

@MainActor @Observable
public final class UsageStore {
    public private(set) var reports: [UsageReport] = []
    @ObservationIgnored private let backend: any UsageBackend
    @ObservationIgnored private var watch: Task<Void, Never>?

    public init(backend: any UsageBackend) {
        self.backend = backend
        load()
        watch = Task { [backend, weak self] in
            for await _ in backend.changes() { self?.load() }
        }
    }

    private func load() {
        let backend = backend
        Task { reports = await backend.reports() }
    }
}

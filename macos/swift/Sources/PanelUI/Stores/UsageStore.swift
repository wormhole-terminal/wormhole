// The Usage tab's state: what each agent last reported of its plan's limits. The reports are the account's, one value
// for every window; the words that change with time ("resets 34m") are made by the view from `now`.

import Foundation
import Observation
import PanelModel

@MainActor @Observable
public final class UsageStore {
    public private(set) var reports: [UsageReport] = []
    /// The clock the words are made from (Unix seconds), moved at each word's own boundary while the tab is on screen.
    public private(set) var now = Int(Date().timeIntervalSince1970)
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
        Task {
            reports = await backend.reports()
            now = Int(Date().timeIntervalSince1970)
        }
    }

    /// Keeps `now` where every word reads right: each second in a limit's last minute ("resets 45s"), each report's minute for
    /// "Updated 3m ago", the reset itself. Runs as the view's task, so nothing ticks while the tab is hidden (the clock is
    /// read again when it shows). With nothing moving it looks again in a minute.
    public func keepTime() async {
        while !Task.isCancelled {
            let date = Date()
            now = Int(date.timeIntervalSince1970)
            let wait = backend.nextChange(from: date) ?? .seconds(60)
            // one-shot: the next moment a word reads differently, which core says
            try? await Task.sleep(for: max(wait, .milliseconds(50)))
        }
    }
}

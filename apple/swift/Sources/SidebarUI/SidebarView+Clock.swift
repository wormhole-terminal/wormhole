// The one timer of the list's words. The second lines read "Working · 3m", a command's clock, "Finished · 12m ago": each
// changes at a moment the wording knows (`SidebarWording.nextChange`), so one sleep is armed until the nearest of them
// and re-armed when it fires or the rows change. Nothing polls, and a list with nothing that moves sleeps.

import SwiftUI
import WorkspaceModel

enum SidebarClock {
    /// When the nearest of `terminals`' second lines next reads differently; nil when none moves.
    static func nextChange(of terminals: [TerminalSummary], now: Date) -> Date? {
        terminals.compactMap { SidebarWording.nextChange(for: $0, now: now) }.min().map { now.addingTimeInterval($0) }
    }
}

extension SidebarView {
    /// When the nearest second line on screen next reads differently; nil with none that moves or under a fixed clock.
    var nextChange: Date? {
        guard appearance.now == nil else { return nil }
        return SidebarClock.nextChange(of: SidebarListing.sections(for: state).flatMap(\.rows).map(\.terminal), now: now)
    }

    func waitForNextChange() async {
        guard let due = nextChange else { return }
        // A hair past the instant, so the words are read after they change.
        try? await Task.sleep(for: .seconds(max(due.timeIntervalSinceNow, 0) + 0.01))
        guard !Task.isCancelled else { return }
        now = Date()
    }
}

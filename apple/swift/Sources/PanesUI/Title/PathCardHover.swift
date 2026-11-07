// When the path's hover card is up: resting on the folder for a moment opens it, and it stays while the
// pointer is on it or on the folder and closes a moment after it has left both, so crossing the gap between
// them doesn't close it (`window/title/path_card.rs`, 400 ms to open and 200 ms to close).

import Observation
import SwiftUI

@MainActor @Observable
final class PathCardHover {
    enum Over { case path, card }

    static let openDelay: Duration = .milliseconds(400)
    static let closeDelay: Duration = .milliseconds(200)

    var shown = false
    @ObservationIgnored private var overPath = false
    @ObservationIgnored private var overCard = false
    @ObservationIgnored private var timer: Task<Void, Never>?

    func hover(_ over: Over, _ inside: Bool) {
        if over == .path { overPath = inside } else { overCard = inside }
        let want = overPath || overCard
        // Nothing to change: a timer that was about to do the opposite is no longer wanted.
        guard want != shown else { timer?.cancel(); timer = nil; return }
        timer?.cancel()
        timer = Task { [weak self] in
            try? await Task.sleep(for: want ? Self.openDelay : Self.closeDelay)
            guard !Task.isCancelled, let self else { return }
            shown = overPath || overCard
        }
    }

    /// Back to nothing: the folder was clicked or the terminal changed under the pointer.
    func close() {
        timer?.cancel()
        (shown, overPath, overCard) = (false, false, false)
    }
}

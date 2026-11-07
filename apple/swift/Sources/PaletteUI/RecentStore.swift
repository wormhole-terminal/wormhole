// What was chosen in the palette lately, newest first, for its Recent group: terminals and folders (actions are
// suggested by what fits the window instead). Kept for the run of the app, as the GPUI palette keeps it.

import Foundation
import Observation
import PaletteModel

@MainActor @Observable
final class RecentStore {
    static let shared = RecentStore()
    static let limit = 6

    private(set) var ids: [String] = []

    func remember(_ command: Command) {
        guard command.group != .actions else { return }
        ids.removeAll { $0 == command.id }
        ids.insert(command.id, at: 0)
        ids = Array(ids.prefix(Self.limit))
    }
}

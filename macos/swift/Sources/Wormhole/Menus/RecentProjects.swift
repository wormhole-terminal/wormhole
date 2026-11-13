// The projects worked in lately, most recent first (GPUI's `app_state/recent_projects.rs`): Open Recent in the menu bar. A project
// counts once a terminal in front is in it; the list is a file in the data folder, so a test instance keeps its own.

import Foundation
import PlatformModel
import WindowUI

@MainActor
final class RecentProjects {
    private static let limit = 8
    private let file: URL
    private(set) var roots: [String]

    init(dataDirectory: URL) {
        file = dataDirectory.appendingPathComponent("recent-projects.json")
        roots = (try? Data(contentsOf: file)).flatMap { try? JSONDecoder().decode([String].self, from: $0) } ?? []
    }

    /// `root` was just worked in: it leads the list. True when that changed anything.
    @discardableResult
    func remember(_ root: String) -> Bool {
        guard roots.first != root else { return false }
        roots = Array(([root] + roots.filter { $0 != root }).prefix(Self.limit))
        try? JSONEncoder().encode(roots).write(to: file, options: .atomic)
        return true
    }

    /// What Open Recent calls them: the folder's name.
    var names: [String] { roots.map { ($0 as NSString).lastPathComponent } }
}

extension AppController {
    /// What the menu bar shows of the window in front, with the parts that come from the app: Open Recent, the front terminal's
    /// project actions (the core lists them only for a trusted project) and the shells New Terminal With offers.
    func menuState(for hub: WindowHub) -> MenuState {
        var state = hub.menuState(
            look: sessions.first { $0.hub === hub }?.dress.look ?? look, isFullScreen: window?.styleMask.contains(.fullScreen) ?? false, implemented: router.implemented,
            // The flags of the window's own profile, which is not always the shown one.
            flagsOn: sessions.first { $0.hub === hub }.map(flagsOn) ?? Set(settingsModel.host.flags().filter(\.on).map(\.key)))
        // An action of a feature that is off in the window's profile is left out (`FeatureMask::hidden_actions`).
        if let session = sessions.first(where: { $0.hub === hub }) {
            let off = featuresOff(session)
            state.hidden.formUnion(Keymap.shared.entries.filter { $0.feature.map(off.contains) ?? false }.map(\.id))
        }
        if let root = hub.panel.project.state?.root { recents.remember(root) }
        state.recent = recents.names
        state.shells = settingsModel.host.shellChoices().map(\.name)
        state.projectActions = (hub.panel.project.state?.actions ?? []).map {
            // Its key as the core wrote it for the row ("⌃⌘T"); the core lists an action only while it may run.
            .init(id: $0.id, title: $0.title, key: $0.key.flatMap { KeySpec(glyphs: $0) })
        }
        return state
    }
}

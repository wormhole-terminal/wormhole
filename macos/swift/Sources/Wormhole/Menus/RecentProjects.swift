// The projects worked in lately, most recent first, one list per profile (GPUI's `app_state/recent_projects.rs`, kept in the
// profile's own database): Open Recent in the menu bar, the status item, the palette and the welcome's places. A project counts once
// a terminal in front is in it. A window reads its own profile's list, whichever profile Wormhole is in.

import Foundation
import PanelModel
import PlatformModel
import WindowUI

#if WITH_UNIFFI
    import PlatformBridge
#endif

@MainActor
final class RecentProjects {
    /// What a profile's list was when it was first asked for, kept up to date as projects are remembered.
    private var lists: [String: [String]] = [:]
    /// Whether the lists live in the profiles' databases (a real window); a sample keeps them in memory.
    private let stored: Bool

    init(real: Bool) { stored = real }

    func roots(of profile: String) -> [String] {
        if let known = lists[profile] { return known }
        var loaded: [String] = []
        #if WITH_UNIFFI
            if stored { loaded = PlatformBridge.recentProjects(profile: profile) }
        #endif
        lists[profile] = loaded
        return loaded
    }

    /// `root` was just worked in `profile`: it leads that profile's list. True when that changed anything.
    @discardableResult
    func remember(_ root: String, profile: String) -> Bool {
        guard roots(of: profile).first != root else { return false }
        #if WITH_UNIFFI
            if stored {
                lists[profile] = PlatformBridge.rememberRecentProject(root, profile: profile)
                return true
            }
        #endif
        lists[profile] = Array(([root] + roots(of: profile).filter { $0 != root }).prefix(8))
        return true
    }

    /// What Open Recent calls them: the folder's name as it may be drawn (a root is a key where its name is not text).
    func names(of profile: String) -> [String] { roots(of: profile).map(Self.name) }

    /// A root's folder name as it may be drawn: the bytes that are not text read U+FFFD (`DiskName`).
    static func name(_ root: String) -> String { DiskName.shown((root as NSString).lastPathComponent) }
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
            // Show Project also needs project folders on, as well as a folder (`features/project/def.rs` `menu`): its key
            // does nothing while they are off.
            if off.contains("project") { state.disabled.insert("showProject") }
        }
        // Terminals that outlive the app are the shown profile's switch, not the window's (`persistent_terminals_on`).
        state.persistent = settingsModel.host.flags().contains { $0.key == "persistent_terminals" && $0.on }
        // The window's own profile's list, which is not always the shown one.
        let profile = sessions.first { $0.hub === hub }?.windows.profile ?? windowsHost.profile
        if let root = hub.panel.project.state?.root { recents.remember(root, profile: profile) }
        state.recent = recents.names(of: profile)
        state.shells = settingsModel.host.shellChoices().map(\.name)
        state.projectActions = (hub.panel.project.state?.actions ?? []).map {
            // Its key as the core wrote it for the row ("⌃⌘T"); the core lists an action only while it may run.
            .init(id: $0.id, title: $0.title, key: $0.key.flatMap { KeySpec(glyphs: $0) })
        }
        return state
    }
}

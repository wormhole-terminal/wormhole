// Which look a window wears: the app's own for a window of the profile Wormhole is in, and for any other profile that
// profile's `ProfileLook`, made the first time a window of it is open and kept while one is. Moving a window to another
// profile changes its dress in place, and the terminals it has are dressed again in the fonts and colours of where it went.

import DesignSystem
import Foundation
import SettingsModel
import WindowUI
import WorkspaceModel

#if WITH_UNIFFI
    import SettingsBridge
    import UniFFISource
    import WormholeBindings
#endif

extension AppController {
    /// The look of profile `id` for a window that is not in the profile Wormhole is in; nil for that profile itself, and
    /// where another profile's settings cannot be read (the window then wears the app's look, as before).
    func profileLook(for id: String) -> ProfileLook? {
        guard id != windowsHost.profile else { return nil }
        if let known = profileLooks[id] { return known }
        #if WITH_UNIFFI
            guard options.isReal, let kit = try? SettingsBridge.make(profile: id) else { return nil }
            let themes = ThemeStore(follows: true)
            themes.resolver = FfiThemeResolver()
            // The profile's own theme files, beside its `config.json`.
            themes.themesDirectory = URL(fileURLWithPath: kit.host.handle.paths().configFile).deletingLastPathComponent().appendingPathComponent("themes")
            let made = ProfileLook(profile: id, app: self, backend: kit.backend, themes: themes) { [weak self] key in
                self?.profileFlags.on(profile: id).contains(key) ?? false
            }
            profileLooks[id] = made
            made.start()
            return made
        #else
            return nil
        #endif
    }

    /// The settings a window reads and writes: its own profile's, whichever profile Wormhole is in.
    func settingsBackend(of session: WindowSession) -> any SettingsBackend {
        profileLook(for: session.windows.profile)?.backend ?? settingsModel.backend
    }

    /// Dress `session` for the profile it is in (as it is made, and again after it moved to another profile).
    func dressForProfile(_ session: WindowSession) {
        let own = profileLook(for: session.windows.profile)
        let look = own?.look ?? self.look
        let terminals = own?.terminalLook ?? terminalLook
        if session.dress.look !== look { session.dress.look = look }
        if session.factory.look !== terminals {
            session.factory.look = terminals
            // The terminals it already has take the fonts, colours and rules of where the window is now.
            session.hub.terminals.forEach { terminals.dress($0) }
        }
        if let own { own.dress(session) } else { settingsApplier?.dress(session) }
        dropUnusedProfileLooks()
    }

    /// A profile with no window left on its own look stops being followed.
    func dropUnusedProfileLooks() {
        for (id, own) in profileLooks where own.sessions.isEmpty { profileLooks[id] = nil }
    }

    /// The windows that wear the app's own look: the ones of the profile Wormhole is in (and the parked ones of a
    /// profile it left, out of sight until it comes back to it).
    var ownSessions: [WindowSession] { sessions.filter { $0.dress.look === look } }
}

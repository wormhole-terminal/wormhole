// Profiles in the app: the tab bar's chip and the menu's Profile entries over the core's profiles (`ProfileList` behind
// `profiles.json`, through the Settings host), and the switch. Which profile Wormhole is in is the core's (every object it
// makes for a window reads it), so a switch is: write it down, follow it with the settings and the theme, and show the
// target's windows (restored the first time, as they were after that) in place of the ones leaving, which are parked
// with their terminals running, as the GPUI app does (`AppState::switch_profile`).

import AppKit
import PanesUI
import WindowUI
import WorkspaceModel
#if WITH_UNIFFI
    import SettingsBridge
    import WormholeBindings
#endif

extension AppController {
    /// The profiles the chip offers a window: Wormhole is in `current`, the window is in its own.
    func profileList(for session: WindowSession) -> @MainActor () -> [ProfileChoice] {
        { [weak self, weak session] in
            #if WITH_UNIFFI
                guard let self, let session, let rows = try? (self.settingsModel.host as? RustSettingsHost)?.profiles() else { return [] }
                return rows.map {
                    ProfileChoice(id: $0.id, name: $0.name, letter: $0.name.first ?? "?", current: $0.isActive, here: $0.id == session.windows.profile)
                }
            #else
                return []
            #endif
        }
    }

    func profileActions() -> ProfileActions {
        ProfileActions(
            switchTo: { [weak self] in self?.switchProfile(to: $0) },
            newProfile: { [weak self] in self?.showProfiles(new: true) },
            manage: { [weak self] in self?.showProfiles() },
            settings: { [weak self] in self?.showSettings() })
    }

    /// Settings' Profile page, where a profile is made (`new`) and managed.
    func showProfiles(new: Bool = false) {
        showSettings()
        settingsController?.show(section: "profile")
        if new { settingsModel.showsNewProfile = true }
    }

    /// The menu's choice by the row's place in the list it was made from.
    func switchProfile(index: Int) {
        #if WITH_UNIFFI
            if let rows = try? (settingsModel.host as? RustSettingsHost)?.profiles(), rows.indices.contains(index) { switchProfile(to: rows[index].id) }
        #endif
    }

    /// Wormhole goes into profile `id`: the windows of the one it leaves go out of sight with their terminals running, and
    /// the target's come back as they were, or from its database the first time, or one new window when it had none.
    func switchProfile(to id: String) {
        #if WITH_UNIFFI
            guard id != windowsHost.profile, let beside = windowsHost.beside else { return }
            let settingsOpen = settingsController?.window?.isVisible == true
            do {
                let target = try profileHosts[id] ?? beside(id)
                // What a relaunch restores: the profile being left, as it is now.
                shown.forEach { $0.saver?.saveNow() }
                let leaving = shown
                try (settingsModel.host as? RustSettingsHost)?.switchProfile(to: id)
                profileHosts[id] = target
                windowsHost = target
                let back = shown
                if back.isEmpty { try openSavedWindows() } else { back.forEach(show) }
                // After the windows exist: applying the settings reaches the shown ones (and the menus read the window in front).
                followProfileSettings()
                leaving.forEach { $0.window?.orderOut(nil) }
                lastFront = shown.last
                if policy.touchesSystem { front.window?.makeKeyAndOrderFront(nil) }
                // The Settings window follows the new profile's: it is made again over the new model.
                settingsController?.window?.close()
                settingsController = nil
                if settingsOpen { showSettings() }
                menus?.windowsChanged()
            } catch {
                devctl?.append("switch-profile \(id) failed: \(error)")
                FileHandle.standardError.write(Data("profile: cannot switch to \(id) (\(error))\n".utf8))
            }
        #endif
    }

    /// A parked window comes back: its chip names the profile Wormhole is in now (the call redraws it).
    private func show(_ session: WindowSession) {
        session.hub.actions.profiles = profileList(for: session)
        if options.background { session.window?.orderBack(nil) } else { session.window?.orderFront(nil) }
    }

    /// The new profile's settings and theme files, applied to the window as the launch's were.
    private func followProfileSettings() {
        settingsModel = AppSettings.makeModel(app: self)
        configureThemes()
        let applier = SettingsApplier(app: self, model: settingsModel)
        settingsApplier = applier
        applier.start()
        applier.applyAmbient()
    }
}

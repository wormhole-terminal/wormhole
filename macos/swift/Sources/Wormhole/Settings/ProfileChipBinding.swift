// Profiles in the app: the tab bar's chip and the menu's Profile entries over the core's profiles (`ProfileList` behind
// `profiles.json`, through the Settings host), and the switch. Which profile Wormhole is in is the core's (every object it
// makes for a window reads it), so a switch is: write it down, follow it with the settings and the theme, and show the
// target's windows (restored the first time, as they were after that) in place of the ones leaving, which are parked
// with their terminals running, as the GPUI app does (`AppState::switch_profile`).

import AppKit
import DesignSystem
import PanesUI
import SidebarUI
import WindowUI
import WorkspaceModel
#if WITH_UNIFFI
    import FilesBridge
    import SettingsBridge
    import WorkspaceBridge
    import WormholeBindings
#endif

extension AppController {
    /// The profiles the chip offers a window: Wormhole is in `current`, the window is in its own.
    func profileList(for session: WindowSession) -> @MainActor () -> [ProfileChoice] {
        { [weak self, weak session] in
            #if WITH_UNIFFI
                guard let self, let session, let rows = try? (self.settingsModel.host as? RustSettingsHost)?.profiles() else { return [] }
                // A profile's colour as this window's own theme draws it (`odd_profile`: the tokens of the window's profile).
                let tokens = session.dress.look.themes.tokens
                return rows.map {
                    ProfileChoice(
                        id: $0.id, name: $0.name, letter: $0.name.first ?? "?",
                        colour: GroupColor(key: $0.color).map { $0.mark(tokens).color }, current: $0.isActive,
                        here: $0.id == session.windows.profile)
                }
            #else
                return []
            #endif
        }
    }

    func profileActions(for session: WindowSession) -> ProfileActions {
        ProfileActions(
            switchTo: { [weak self] in self?.switchProfile(to: $0) },
            openWindow: { [weak self] in self?.openWindow(inProfile: $0) },
            moveWindow: { [weak self, weak session] id in session.map { self?.move($0, toProfile: id) } },
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
                // What a relaunch restores: the profile being left, as it is now. Windows opened in another profile stay in sight.
                let leaving = windows(of: windowsHost.profile)
                leaving.forEach { $0.saver?.saveNow() }
                try (settingsModel.host as? RustSettingsHost)?.switchProfile(to: id)
                profileHosts[id] = target
                windowsHost = target
                // What agents report goes to the profile Wormhole is in; each window still draws its own profile's.
                (sessions.first?.panelBackends.usage as? RustUsageBackend)?.setShown(profile: id)
                aux.startAccess(profile: id)
                leaving.forEach { $0.parked = true }
                let back = windows(of: id)
                back.forEach { $0.parked = false }
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
                devctl?.append("switch-profile \(id) failed: \(error)") // text: a line for a test driver
                FileHandle.standardError.write(Data("profile: cannot switch to \(id) (\(error))\n".utf8)) // text: a line on stderr for whoever launched the app
                // A profile whose data folder cannot be opened saves nothing: the log and the chip say so and why (GPUI's
                // `data_folder::record`).
                let folder = id == "default" ? dataDirectory.path : dataDirectory.appendingPathComponent("profiles/\(id)").path
                LaunchLogging.log("error", "persistence disabled: profile \(id), folder \(folder): \(error)")
                hub.notices.post(.warned(.dataFolder), detail: NoticeReason.dataFolder(profile: id, folder: id == "default" ? dataDirectory.path : dataDirectory.appendingPathComponent("profiles/\(id)").path, error: "\(error)"))
            }
        #endif
    }

    /// Settings → Delete profile, before core removes the folder: Wormhole leaves the profile when it is in it, and its windows end
    /// with their terminals (the GPUI app's `delete_profile`). Not through `shouldClose`: that would quit the app for the last window.
    func end(profile id: String) {
        #if WITH_UNIFFI
            guard id != "default" else { return }
            if windowsHost.profile == id { switchProfile(to: "default") }
            for session in windows(of: id) {
                session.saver?.cancel()
                session.hub.terminals.shutdown()
                session.windows.backend.forget(session.id)
                sessions.removeAll { $0 === session }
                session.window?.delegate = nil
                session.window?.close()
            }
            profileHosts[id] = nil
            profileLooks[id] = nil
            if lastFront == nil || !sessions.contains(where: { $0 === lastFront }) { lastFront = shown.last }
            menus?.windowsChanged()
        #endif
    }

    /// Every window of profile `id`, in sight or parked.
    private func windows(of id: String) -> [WindowSession] { sessions.filter { $0.windows.profile == id } }

    /// "Open Window in <profile>": a window in `id` beside the ones of the profile Wormhole is in, which stays where it is
    /// (the odd window, as the GPUI app has it); the profile Wormhole is in gets its own window the same way.
    func openWindow(inProfile id: String) {
        #if WITH_UNIFFI
            // The picked profile's own host (the shown one's too: `profileHosts` holds it), not the front window's: that is ⌘N's road.
            guard let target = hostBeside(id) else { return }
            // Its own start folder (the profile's), not the one of the window in front.
            let planned = target.backend.plan(from: nil, folder: nil)
            openPlanned(window: planned.window, activate: true, in: target)
        #endif
    }

    /// "Move Window to <profile>": `session`'s window and its running terminals go to `id`, which keeps its record from now on.
    func move(_ session: WindowSession, toProfile id: String) {
        #if WITH_UNIFFI
            let old = session.windows
            guard id != old.profile, let target = hostBeside(id) else { return }
            let frame = frame(of: session)
            guard old.backend.rehome(window: session.id, workspace: session.hub.store.source, frame: frame, to: target.backend) else {
                devctl?.append("move-window \(session.id) to \(id) failed: its record could not be written there") // text: a line for a test driver
                return
            }
            session.windows = target
            session.notes?.follow(profile: id)
            (session.hub.store.source as? RustWorkspaceBackend)?.followProfile(id)
            // What the window reads of its profile follows: the trust its project actions run under and the usage meters
            // (agent access is judged by the window's own profile in the core, which `rehome` told). The panel's
            // backends were made for the profile it left.
            (session.panelBackends.project as? RustProjectBackend)?.service.followProfile(profile: id)
            (session.panelBackends.usage as? RustUsageBackend)?.follow(profile: id)
            (session.panelBackends.files as? RustFilesBackend)?.followProfile(id)
            // The worktrees a window makes, and their ports, are remembered in its own profile's database.
            session.panelBackends.worktrees.followProfile(id)
            session.saver?.windows = target.backend
            // Not left without a window: the profile Wormhole is in keeps one to type in.
            if windows(of: windowsHost.profile).isEmpty {
                let planned = windowsHost.backend.plan(from: nil, folder: nil)
                openPlanned(window: planned.window, activate: false)
            }
            session.hub.actions.profiles = profileList(for: session)
            // Its settings, theme and fonts are the ones of the profile it is in now.
            dressForProfile(session)
            menus?.windowsChanged()
        #endif
    }

    /// The menu's "Open Window in" and "Move Window to" choices, by the row's place in the list they were made from.
    func openWindow(inProfileIndex index: Int) { profileID(at: index).map(openWindow(inProfile:)) }
    func moveFrontWindow(toProfileIndex index: Int) { profileID(at: index).map { move(front, toProfile: $0) } }

    private func profileID(at index: Int) -> String? {
        #if WITH_UNIFFI
            guard let rows = try? (settingsModel.host as? RustSettingsHost)?.profiles(), rows.indices.contains(index) else { return nil }
            return rows[index].id
        #else
            return nil
        #endif
    }

    /// The windows of profile `id`, open beside the shown profile's (made the first time and kept).
    private func hostBeside(_ id: String) -> WindowsHost? {
        if let known = profileHosts[id] { return known }
        guard let beside = windowsHost.beside, let made = try? beside(id) else { return nil }
        profileHosts[id] = made
        aux.startAccess(profile: id)
        return made
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

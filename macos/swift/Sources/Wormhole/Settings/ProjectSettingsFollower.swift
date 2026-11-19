// Keeps Settings layered with the project of the terminal in front (`features/project/detect.rs`
// `apply_project_settings`): whenever another terminal is selected, or its folder changes, or the project's files or
// trust change, the core is asked what the project's `.wormhole/config.json` sets (only while its Settings switch is on
// and it is trusted) and Settings shows those values over the person's, with the rows it sets marked.
//
// They go over the settings of the profile Wormhole is in, so only a window of that profile puts any on, read through
// that window's own project service (its profile's trust and switches): "a project another profile trusts does not
// change the look and behaviour of this one" (`detect.rs:83-89`). The Settings host is the one in force now, which a
// profile switch makes again (`followProfileSettings`).

import Foundation

/// Which window's project is laid over the shown profile's settings and theme.
enum ProjectSettingsSource {
    /// The window the person used last when it is of the profile Wormhole is in, else the one read before while it
    /// still is, else none: never a window of another profile, in front or not. `profileInSight` is a window's profile
    /// while it is open and in sight, nil for one closed or parked.
    static func window<Window>(front: Window?, readBefore: Window?, shown: String, profileInSight: (Window) -> String?) -> Window? {
        [front, readBefore].compactMap { $0 }.first { profileInSight($0) == shown }
    }
}

#if WITH_UNIFFI
    import DesignSystem
    import FilesBridge
    import Observation
    import PanelModel
    import SettingsBridge
    import SettingsModel
    import WindowUI
    import WorkspaceModel

    @MainActor
    final class ProjectSettingsFollower {
        private unowned let app: AppController
        /// What is laid now: the project of this window's terminal in front, over this Settings host.
        private weak var window: WindowSession?
        private weak var host: RustSettingsHost?
        private var folder: String?
        private var watching: Task<Void, Never>?
        /// Counts the looks: only the newest one's observation looks again, so the ones of a window left die out.
        private var look = 0

        init(app: AppController) { self.app = app }

        func start() { follow() }

        /// The terminal in front of the window to read, again whenever it or its folder changes, and whenever the app
        /// says the window in front, the profile Wormhole is in or a window's profile changed.
        func follow() {
            // The sample has no project and no settings of its own to layer.
            guard let host = app.settingsModel.host as? RustSettingsHost else { return }
            let window = source()
            look += 1
            let mine = look
            // The folder as the panel reads it: the core's full path, a key where a name is not text. Never the drawn
            // folder: `~`-shortened and with U+FFFD for every byte that is not text, it names no folder there, or a
            // planted sibling (`caf\u{FFFD}/.wormhole` beside `caf\xE9/`) whose settings would be laid over yours.
            let selected = withObservationTracking { window?.hub.state.selectedPath } onChange: { [weak self] in
                Task { @MainActor in
                    guard let self, self.look == mine else { return }
                    self.follow()
                }
            }
            guard window !== self.window || host !== self.host || selected != folder else { return }
            (self.window, self.host, folder) = (window, host, selected)
            watching?.cancel()
            watching = nil
            reload()
            guard let selected, let project = window?.panelBackends.project as? RustProjectBackend else { return }
            // Its files and its trust: either may change what the project sets.
            let changes = project.changes(at: selected)
            watching = Task { [weak self] in
                for await _ in changes { self?.reload() }
            }
        }

        private func source() -> WindowSession? {
            let open = app.sessions
            return ProjectSettingsSource.window(front: app.lastFront ?? app.shown.first, readBefore: window, shown: app.windowsHost.profile) { session in
                open.contains { $0 === session } && !session.parked ? session.windows.profile : nil
            }
        }

        /// Whether what was read through `window` for `folder` may still be laid on `host`: the same window, terminal
        /// and Settings as when the read began, and the window still one of the profile Wormhole is in.
        private func stillReads(_ window: WindowSession, _ folder: String, on host: RustSettingsHost) -> Bool {
            self.window === window && self.folder == folder && self.host === host
                && (app.settingsModel.host as? RustSettingsHost) === host && source() === window
        }

        private func reload() {
            guard let host else { return }
            guard let window, let folder, let project = window.panelBackends.project as? RustProjectBackend else {
                ThemeStore.shared.projectTheme = nil
                app.settingsModel.projectTheme = nil
                return host.follow(project: nil)
            }
            Task {
                let found = await project.settingsOverrides(at: folder)
                let theme = await project.themeOverlay(at: folder)
                // The terminal, the window in front or a profile may have changed while the core read: what it read was
                // judged by the trust of the profile the window was in then, and is dropped.
                guard stillReads(window, folder, on: host) else { return }
                host.follow(project: found)
                // The project's theme colours the windows while one of its terminals is in front. The mark names the
                // project's root as a key, which the Project folders page selects it by.
                ThemeStore.shared.projectTheme = theme
                app.settingsModel.projectTheme = theme == nil ? nil
                    : ProjectThemeMark(name: found?.name ?? DiskName.shown((folder as NSString).lastPathComponent), root: found?.root ?? folder)
            }
        }
    }
#endif

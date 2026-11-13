// Keeps Settings layered with the project of the terminal in front (`features/project/detect.rs`
// `apply_project_settings`): whenever another terminal is selected, or its folder changes, or the project's files or
// trust change, the core is asked what the project's `.wormhole/config.json` sets (only while its Settings switch is on
// and it is trusted) and Settings shows those values over the person's, with the rows it sets marked.

#if WITH_UNIFFI
    import DesignSystem
    import FilesBridge
    import Foundation
    import Observation
    import SettingsBridge
    import SettingsModel
    import WorkspaceModel

    @MainActor
    final class ProjectSettingsFollower {
        private unowned let app: AppController
        private let project: RustProjectBackend
        private let host: RustSettingsHost
        private var folder: String?
        private var watching: Task<Void, Never>?

        /// Nil unless the window is the core's: the sample has no project and no settings of its own to layer.
        init?(app: AppController) {
            guard let host = app.settingsModel.host as? RustSettingsHost,
                let project = app.panelBackends.project as? RustProjectBackend
            else { return nil }
            (self.app, self.project, self.host) = (app, project, host)
        }

        func start() { follow() }

        /// The terminal in front, and again whenever it or its folder changes.
        private func follow() {
            let selected = withObservationTracking { selectedFolder } onChange: { [weak self] in
                Task { @MainActor in self?.follow() }
            }
            guard selected != folder else { return }
            folder = selected
            reload()
            watching?.cancel()
            guard let selected else { return }
            // Its files and its trust: either may change what the project sets.
            let changes = project.changes(at: expanded(selected))
            watching = Task { [weak self] in
                for await _ in changes { self?.reload() }
            }
        }

        /// The folder of the terminal in front, as the core's rows show it.
        private var selectedFolder: String? {
            let state = app.hub.state
            return (state.terminals.first { $0.id == state.selected } ?? state.all.first { $0.id == state.selected })?.folder
        }

        private func reload() {
            guard let folder else {
                ThemeStore.shared.projectTheme = nil
                app.settingsModel.projectTheme = nil
                return host.follow(project: nil)
            }
            let path = expanded(folder)
            Task {
                let found = await project.settingsOverrides(at: path)
                let theme = await project.themeOverlay(at: path)
                // The terminal may have changed while the core read.
                guard self.folder == folder else { return }
                host.follow(project: found)
                // The project's theme colours the windows while one of its terminals is in front.
                ThemeStore.shared.projectTheme = theme
                app.settingsModel.projectTheme = theme == nil ? nil : ProjectThemeMark(name: found?.name ?? (path as NSString).lastPathComponent, root: path)
            }
        }

        private func expanded(_ folder: String) -> String { (folder as NSString).expandingTildeInPath }
    }
#endif

// `SettingsHost` over core's lists and the platform's closures. Core answers what it can headlessly (the shell and
// icon theme selects, the folders, resetting everything, the agents' hooks, the terminals to import from, the
// profiles); what only the app can do (the default terminal, notifications, the update check, the installed apps,
// system fonts) stays a closure the app passes in. The records are read off the main thread and kept: a page asks for
// them on every draw, and reading them stats every PATH entry and parses other terminals' files.

import AppKit
import Observation
import SettingsModel
import WormholeBindings

@MainActor @Observable
public final class RustSettingsHost: SettingsHost {
    /// What only the app can do.
    public struct Platform {
        /// A button core does not answer: the default terminal, notification permission and test, the update check,
        /// the tips, the new-profile form, developer tools.
        public var perform: @MainActor (SettingsAction) -> Void = { _ in }
        public var fontFamilies: @MainActor () -> [String] = { [] }
        /// The "Open in" select: the apps installed (a system list).
        public var openApps: @MainActor () -> [SettingOption] = { [] }
        public var open: @MainActor (URL) -> Void = { NSWorkspace.shared.open($0) }
        public init() {}
    }

    public let handle: SettingsHandle
    /// Told when core changed settings by itself (Reset all), so the model does not wait for a wake.
    let backend: RustSettingsBackend
    let platform: Platform
    let importHome: String?
    private var records: PanelRecords
    /// Why the last hook install or removal failed (core's sentence); nil after one that worked.
    public private(set) var failure: String?

    init(handle: SettingsHandle, backend: RustSettingsBackend, platform: Platform, importHome: String?) {
        (self.handle, self.backend, self.platform, self.importHome) = (handle, backend, platform, importHome)
        let profile = RecordMapping.profile((try? settingsProfiles()) ?? [])
        records = PanelRecords(
            agents: [], imports: [],
            profile: profile ?? ProfileRecord(name: "", startFolder: "~", gitName: "", gitEmail: ""))
        refreshAgents()
        refreshImports()
    }

    public func options(for source: OptionSource) -> [SettingOption] {
        switch source {
        case .shells: handle.shellOptions().map(RecordMapping.option)
        case .iconThemes: handle.iconThemeOptions().map(RecordMapping.option)
        case .openIn: platform.openApps()
        }
    }

    public func fontFamilies() -> [String] { platform.fontFamilies() }

    public func panelRecords() -> PanelRecords { records }

    public func perform(_ action: SettingsAction) {
        let paths = handle.paths()
        switch action {
        case .openConfig: platform.open(URL(fileURLWithPath: paths.configFile))
        case .openData: platform.open(URL(fileURLWithPath: paths.dataDir))
        case .openLogs: platform.open(URL(fileURLWithPath: paths.logsDir))
        case .openFontsFolder: platform.open(URL(fileURLWithPath: paths.fontsDir))
        case .openIconThemesFolder: platform.open(URL(fileURLWithPath: paths.iconThemesDir))
        case .resetAll:
            _ = handle.resetAll()
            backend.reload()
        case .rescanAgents: refreshAgents()
        case .scanImports: refreshImports()
        default: platform.perform(action)
        }
    }

    public func perform(_ hooks: AgentRecord.HookAction, agent id: String) {
        Task.detached {
            let result = Result { try settingsSetAgentHooks(agent: id, install: hooks != .remove) }
            await MainActor.run {
                self.failure = result.failure.map(SettingsFailure.sentence)
                self.refreshAgents()
            }
        }
    }

    /// Reads the agents and their hooks again (after an install, a removal, Rescan PATH).
    public func refreshAgents() {
        Task.detached {
            let rows = settingsAgentRows().map(RecordMapping.agent)
            await MainActor.run { self.records.agents = rows }
        }
    }

    /// Looks for other terminals' settings again (Scan again).
    public func refreshImports() {
        let home = importHome
        Task.detached {
            let rows = settingsImportRows(home: home).map(RecordMapping.imported)
            await MainActor.run { self.records.imports = rows }
        }
    }

    /// Reads the profile record again, after a switch or an edit of the profile.
    public func refreshProfile() {
        if let profile = RecordMapping.profile((try? settingsProfiles()) ?? []) { records.profile = profile }
    }
}

extension Result {
    fileprivate var failure: Failure? {
        if case .failure(let error) = self { error } else { nil }
    }
}

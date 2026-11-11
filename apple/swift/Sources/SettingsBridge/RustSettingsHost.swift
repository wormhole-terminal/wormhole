// `SettingsHost` over core's lists and the platform's closures. Core answers what it can headlessly (the shell and
// icon theme selects, the folders, resetting everything, the agents' hooks, the terminals to import from, the
// profiles); what only the app can do (the default terminal, notifications, the update check, the installed apps,
// system fonts) stays a closure the app passes in. The records are read off the main thread and kept: a page asks for
// them on every draw, and reading them stats every PATH entry and parses other terminals' files.

import AppKit
import FontPickerModel
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
        /// The bundled font's files (JetBrains Mono's four), so its name draws in its own face; none if the app has no folder.
        public var bundledFonts: @MainActor () -> [URL] = { [] }
        /// Whether Wormhole is the app macOS opens scripts with; nil without a bundle to name.
        public var isDefaultTerminal: @MainActor () -> Bool? = { nil }
        /// The "Open in" select: the apps installed (a system list).
        public var openApps: @MainActor () -> [SettingOption] = { [] }
        public var open: @MainActor (URL) -> Void = { NSWorkspace.shared.open($0) }
        /// The programs let in (Agent access): the core's desk lists them, in the order Settings shows.
        public var approvals: @MainActor () -> [AccessApprovalRecord] = { [] }
        /// Takes an approval back (`stored`: a database row; else one held in memory): the hook server ends the
        /// streams it let in, which only the app can reach.
        public var revokeApproval: @MainActor (_ number: UInt64, _ stored: Bool) -> Void = { _, _ in }
        /// The project service the Project tab uses, so trust given in Settings is what the tab shows; nil: no Project folders page.
        public var projects: @MainActor () -> ProjectService? = { nil }
        /// Asked before a Probe that needs it is approved (Touch ID, or the password); nil: no Extensions page.
        public var owner: @MainActor () -> (any OwnerAuthenticator)? = { nil }
        /// Whether the owner can be asked at all: `Authenticating.canAuthenticate`.
        public var ownerAvailable: @MainActor @Sendable () -> Bool = { true }
        /// The folders of the open terminals, whose projects the Project folders page offers.
        public var projectFolders: @MainActor () -> [String] = { [] }
        /// A new terminal in a project's folder.
        public var openTerminal: @MainActor (String) -> Void = { _ in }
        public init() {}
    }

    public let handle: SettingsHandle
    /// Told when core changed settings by itself (Reset all), so the model does not wait for a wake.
    let backend: RustSettingsBackend
    let platform: Platform
    let importHome: String?
    var records: PanelRecords
    public let probes: (any ProbesBackend)?
    /// Read on first use: the settings model is made before the first window, and the service is the window's.
    @ObservationIgnored public private(set) lazy var projects: (any ProjectsBackend)? = platform.projects().map {
        RustProjectsBackend(
            service: $0, folders: platform.projectFolders, terminal: platform.openTerminal, ownerAvailable: platform.ownerAvailable)
    }
    /// Why the last hook install or removal failed (core's sentence); nil after one that worked.
    public internal(set) var failure: String?

    init(handle: SettingsHandle, backend: RustSettingsBackend, platform: Platform, importHome: String?) {
        (self.handle, self.backend, self.platform, self.importHome) = (handle, backend, platform, importHome)
        if let owner = platform.owner(), let desk = try? ProbesDesk(authenticator: owner) {
            probes = RustProbesBackend(desk: desk)
        } else {
            probes = nil
        }
        let profile = RecordMapping.profile((try? settingsProfiles()) ?? [])
        records = PanelRecords(
            agents: [], imports: [],
            profile: profile ?? ProfileRecord(name: "", startFolder: "~", gitName: "", gitEmail: ""))
        refreshAgents()
        refreshSkills()
        refreshImports()
        refreshProfiles()
        refreshCommandLine()
    }

    public func options(for source: OptionSource) -> [SettingOption] {
        switch source {
        case .shells: handle.shellOptions().map(RecordMapping.option)
        case .iconThemes: handle.iconThemeOptions().map(RecordMapping.option)
        case .openIn: platform.openApps()
        }
    }

    public var fontRules: any FontRules { RustFontRules(fontsDir: handle.paths().fontsDir, bundledFiles: platform.bundledFonts()) }
    public func isDefaultTerminal() -> Bool? { platform.isDefaultTerminal() }

    /// Says why the last button did nothing (nil: it did something), until the next one.
    public func problem(_ sentence: String?) {
        failure = sentence
        records.problem = sentence
    }

    /// The window opened: what the pages read from the disk is read again.
    public func windowOpened() {
        refreshAgents()
        refreshSkills()
        refreshProfiles()
        refreshCommandLine()
        refreshApprovals()
    }

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
        case .rescanAgents:
            refreshAgents()
            refreshSkills()
        case .scanImports: refreshImports()
        case .installCommandLine: installCommandLine()
        case .uninstallCommandLine: uninstallCommandLine()
        default: platform.perform(action)
        }
    }

    public func perform(_ hooks: AgentRecord.HookAction, agent id: String) {
        Task.detached {
            let result = Result { try settingsSetAgentHooks(agent: id, install: hooks != .remove) }
            await MainActor.run {
                self.problem(result.failure.map(SettingsFailure.sentence))
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
        records.scanningImports = true
        Task.detached {
            let rows = settingsImportRows(home: home).map(RecordMapping.imported)
            await MainActor.run { (self.records.imports, self.records.scanningImports) = (rows, false) }
        }
    }

    /// Reads the profile record again, after a switch or an edit of the profile.
    public func refreshProfile() {
        if let profile = RecordMapping.profile((try? settingsProfiles()) ?? []) { records.profile = profile }
    }
}

// The seam to the settings the Rust core holds (`config.json` through `Config::update`): values by key, writes,
// resets and a change notification. The real one arrives with the FFI (`SettingsHandle`); until then a mock
// keeps the values in memory. Keys are `config.json`'s, a path for a key inside an object (`motion.insert`).
// Writes are sanitised on the Rust side, so a caller reads back what stuck.

import FontPickerModel

@MainActor
public protocol SettingsBackend: AnyObject {
    /// The effective values of every key the schema names (a key the file lacks has its default).
    var values: [String: JSONValue] { get }
    func set(_ key: String, to value: JSONValue)
    /// Puts keys back at their defaults (the reset arrow of a row, Reset All).
    func reset(_ keys: [String])
    /// Calls `onChange` after any write, from anywhere (a hand edit of the file, another window).
    func observe(_ onChange: @escaping @MainActor () -> Void) -> SettingsObservation
}

/// Keeps an observation alive; dropping it stops it. `cancel` may run on any thread (a deinit does), so it
/// hands the work back to the main actor itself.
public final class SettingsObservation: Sendable {
    private let cancel: @Sendable () -> Void
    public init(cancel: @escaping @Sendable () -> Void) { self.cancel = cancel }
    deinit { cancel() }
}

/// What the platform provides that is not a stored value: the options of selects that list apps, shells and icon
/// themes, system fonts, and what a button does.
@MainActor
public protocol SettingsHost: AnyObject {
    func options(for source: OptionSource) -> [SettingOption]
    /// The font pickers' rules and sources: core's, or the same in Swift when there is no core.
    var fontRules: any FontRules { get }
    func perform(_ action: SettingsAction)
    /// What the agents, import and profile pages list.
    func panelRecords() -> PanelRecords
    /// Installs, repairs or removes the status hooks of the agent with this id (a row's button).
    func perform(_ hooks: AgentRecord.HookAction, agent id: String)
    /// Takes an approval back (a row's Revoke): the program is asked again, and what it started is closed.
    func revoke(_ approval: ApprovalRecord)
    /// Makes a profile, as a copy of `copyOf`'s settings or with the defaults, and deletes one (never Default). The
    /// reason it was refused, or nil.
    func makeProfile(name: String, copyOf: String?) -> String?
    func removeProfile(_ id: String) -> String?
    /// The Settings window was opened: what a page reads from the disk is read again.
    func windowOpened()
    /// The Probes and the projects behind the Extensions and Project folders pages; nil where there are none (the mock).
    var probes: (any ProbesBackend)? { get }
    var projects: (any ProjectsBackend)? { get }
    /// Installs, updates or removes the Wormhole skill of the agent `id` (a row's button).
    func perform(skill id: String, install: Bool)
    /// The Feature flags page: a switch per flag, flipped, and the unlisted key that unlocks them in a release.
    func flags() -> [FlagRecord]
    func setFlag(_ key: String, on: Bool)
    /// Whether they are unlocked now.
    @discardableResult func toggleFlagsUnlocked() -> Bool
    /// The pages the nav shows for this build and these settings; nil shows every page.
    func shownSections() -> Set<String>?
}

extension SettingsHost {
    public var fontRules: any FontRules { LocalFontRules() }
    /// A host that has no agents to set up (the mock) ignores the button.
    public func perform(_ hooks: AgentRecord.HookAction, agent id: String) {}
    /// A host that holds no approvals (the mock) has none to take back.
    public func revoke(_ approval: ApprovalRecord) {}
    public func makeProfile(name: String, copyOf: String?) -> String? { Self.noProfiles }
    public func windowOpened() {}
    public var probes: (any ProbesBackend)? { nil }
    public var projects: (any ProjectsBackend)? { nil }
    public func perform(skill id: String, install: Bool) {}
    public func flags() -> [FlagRecord] { [] }
    public func setFlag(_ key: String, on: Bool) {}
    public func toggleFlagsUnlocked() -> Bool { false }
    public func shownSections() -> Set<String>? { nil }
    public func removeProfile(_ id: String) -> String? { Self.noProfiles }
    private static var noProfiles: String {
        String(localized: "settings-profile-unavailable", defaultValue: "Profiles are not available in this build.")
    }
}

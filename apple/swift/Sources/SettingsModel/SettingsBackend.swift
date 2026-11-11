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
    /// What is wrong with the file (it does not read, the last save failed), for the Advanced page; nil when nothing.
    func banner() -> String?
}

extension SettingsBackend {
    public func banner() -> String? { nil }
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
    /// The shells a terminal can start with, login shell first; none where there is no core.
    func shellChoices() -> [ShellChoice]
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
    /// The fields of the profile Wormhole is in, and writing them back: core's sentence when it refuses, or nil.
    func profileForm() -> ProfileForm?
    func saveProfile(_ form: ProfileForm) -> String?
    /// One terminal's settings as the Import page lists them (nil: none found), and taking the ticked rows (indexes
    /// into its items, of `shown` rows): the line that says what happened.
    func importReview(_ id: String) async -> ImportReview?
    func applyImport(_ id: String, ticked: [Int], shown: Int) -> (text: String, failed: Bool)
    /// Whether Wormhole is what macOS opens scripts with: nil when this build cannot be (no bundle to name).
    func isDefaultTerminal() -> Bool?
    /// The pages' counted sentences in the catalogs' plurals: "17 more agents Wormhole recognises", "3 of 5 selected".
    func othersHead(count: Int) -> String
    func importSelectedLine(selected: Int, total: Int) -> String
    /// The question before settings go back to their defaults, with how many.
    func resetQuestion(count: Int) -> String
    /// What a preview section says at its top, naming the feature flag it is behind; nil for one that always ships.
    func previewNote(section: String) -> String?
    /// The shell profiles listed under the Shells row, and where worktrees go once a folder is named.
    func shellProfiles() -> [ShellLine]
    func worktreeCaption(base: String) -> String?
    /// A theme file copied into the profile's themes folder and picked: core's reason when it is no theme, or nil.
    func importTheme(path: String) -> String?
    /// What the question before moving the user theme `value` to the Trash says (nil: it has no file), and the move itself
    /// (the picks that name it go back to the defaults first and come back if it fails): whether it was done.
    func themeTrashPlan(_ value: String) -> ThemeTrashQuestion?
    func trashTheme(_ value: String) -> Bool
    /// The header an update check carries (what "What an update check sends" shows): nil without the core.
    func updateUserAgent(version: String, counting: Bool, unstable: Bool) -> String?
    /// "Sharing starts in about 24 hours, …" while this copy waits to be counted; nil once it is (or without the core).
    func updateCountWait() -> String?
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
    public func profileForm() -> ProfileForm? { nil }
    public func shellProfiles() -> [ShellLine] { [] }
    public func shellChoices() -> [ShellChoice] { [] }
    public func worktreeCaption(base: String) -> String? { nil }
    public func isDefaultTerminal() -> Bool? { nil }
    public func othersHead(count: Int) -> String { "\(count) more agent\(count == 1 ? "" : "s") Wormhole recognises" }
    public func importSelectedLine(selected: Int, total: Int) -> String { "\(selected) of \(total) selected" }
    public func previewNote(section: String) -> String? { nil }
    public func resetQuestion(count: Int) -> String { "Reset \(count) changed setting\(count == 1 ? "" : "s") to their default? Your terminals, notes and themes are kept." }
    public func updateUserAgent(version: String, counting: Bool, unstable: Bool) -> String? { nil }
    public func updateCountWait() -> String? { nil }
    public func importTheme(path: String) -> String? { "Themes can't be imported in this build." }
    public func themeTrashPlan(_ value: String) -> ThemeTrashQuestion? { nil }
    public func trashTheme(_ value: String) -> Bool { false }
    public func saveProfile(_ form: ProfileForm) -> String? { Self.noProfiles }
    public func importReview(_ id: String) async -> ImportReview? { nil }
    public func applyImport(_ id: String, ticked: [Int], shown: Int) -> (text: String, failed: Bool) { (Self.noProfiles, true) }
    public func removeProfile(_ id: String) -> String? { Self.noProfiles }
    private static var noProfiles: String {
        String(localized: "settings-profile-unavailable", defaultValue: "Profiles are not available in this build.")
    }
}

/// The question before a theme the person imported goes to the Trash.
public struct ThemeTrashQuestion: Sendable, Equatable {
    public let message: String
    public let detail: String

    public init(message: String, detail: String) { (self.message, self.detail) = (message, detail) }
}

/// A shell profile of `config.json`: its name and what it runs.
public struct ShellLine: Sendable, Equatable {
    public let name: String
    public let commandLine: String

    public init(name: String, commandLine: String) { (self.name, self.commandLine) = (name, commandLine) }
}

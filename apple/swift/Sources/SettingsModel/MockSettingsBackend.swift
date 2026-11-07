// An in-memory backend and host for previews and tests: the schema's defaults, writes that stick as given.

@MainActor
public final class MockSettingsBackend: SettingsBackend {
    public private(set) var values: [String: JSONValue]
    private let defaults: [String: JSONValue]
    private var observers: [Int: @MainActor () -> Void] = [:]
    private var next = 0
    /// The keys written, in order, for tests to read.
    public private(set) var writes: [String] = []

    public init(schema: SettingsSchema = .bundled, overrides: [String: JSONValue] = [:]) {
        defaults = schema.defaults
        values = schema.defaults.merging(overrides) { _, new in new }
    }

    public func set(_ key: String, to value: JSONValue) {
        guard values[key] != value else { return }
        values[key] = value
        writes.append(key)
        observers.sorted { $0.key < $1.key }.forEach { $0.value() }
    }

    public func reset(_ keys: [String]) {
        for key in keys { values[key] = defaults[key] }
        writes.append(contentsOf: keys)
        observers.sorted { $0.key < $1.key }.forEach { $0.value() }
    }

    public func observe(_ onChange: @escaping @MainActor () -> Void) -> SettingsObservation {
        let id = next
        next += 1
        observers[id] = onChange
        return SettingsObservation { [weak self] in
            // `deinit` may run anywhere; the observer table is main-actor state.
            Task { @MainActor in self?.observers[id] = nil }
        }
    }
}

@MainActor
public final class MockSettingsHost: SettingsHost {
    public private(set) var performed: [SettingsAction] = []

    private let records: PanelRecords

    public init(records: PanelRecords = .sample) { self.records = records }

    public func options(for source: OptionSource) -> [SettingOption] {
        switch source {
        case .openIn:
            [("", "Finder"), ("dev.zed.Zed", "Zed"), ("com.microsoft.VSCode", "Visual Studio Code")].map {
                SettingOption(value: .string($0.0), label: $0.1)
            }
        case .shells:
            [("", "Login shell"), ("/bin/zsh", "zsh"), ("/opt/homebrew/bin/fish", "fish")].map {
                SettingOption(value: .string($0.0), label: $0.1)
            }
        case .iconThemes: [SettingOption(value: "Wormhole Icons", label: "Wormhole Icons")]
        }
    }

    public func perform(_ action: SettingsAction) { performed.append(action) }
    public func panelRecords() -> PanelRecords { records }

    /// The sample profile as fields; a save always sticks.
    public func profileForm() -> ProfileForm? {
        ProfileForm(
            id: "default", name: records.profile.name, startDir: records.profile.startFolder == "~" ? "" : records.profile.startFolder,
            gitName: records.profile.gitName, gitEmail: records.profile.gitEmail,
            logins: [.init(agent: "claude", name: "Claude", placeholder: "~/.claude-work", dir: ""), .init(agent: "codex", name: "Codex", placeholder: "~/.codex-work", dir: "")],
            startCaption: "Where a new terminal starts. A folder you add below opens in \(records.profile.name) too.")
    }
    public func saveProfile(_ form: ProfileForm) -> String? { nil }
}

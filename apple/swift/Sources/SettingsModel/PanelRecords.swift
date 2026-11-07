// The records the list-like pages draw (agents and their hooks, terminals found to import from, the profile).
// They come from the host (`SettingsHost`), since each reads the disk or the app's state in Rust; the pages show
// them and send the buttons back as actions.

public struct AgentRecord: Sendable, Equatable, Identifiable {
    /// `ready`: its status hooks are in. `needsRepair`: some are missing. `statusFromOutput`: found, and its
    /// status is read from the terminal (no hooks yet, or no hook support). `notInstalled`: not on this Mac's PATH.
    public enum State: Sendable { case ready, needsRepair, statusFromOutput, notInstalled }
    /// What the one button of a row does to the agent's hooks.
    public enum HookAction: Sendable { case install, repair, remove }
    public let id: String
    public let name: String
    public let detail: String
    public let state: State
    /// The button the row offers, none for an agent that has no hook support.
    public let hooks: HookAction?

    public init(id: String, name: String, detail: String, state: State, hooks: HookAction? = nil) {
        (self.id, self.name, self.detail, self.state, self.hooks) = (id, name, detail, state, hooks)
    }
}

public struct ImportRecord: Sendable, Equatable, Identifiable {
    public let id: String
    public let app: String
    public let summary: String

    public init(id: String, app: String, summary: String) { (self.id, self.app, self.summary) = (id, app, summary) }
}

public struct ProfileRecord: Sendable, Equatable {
    public var name: String
    public var startFolder: String
    public var gitName: String
    public var gitEmail: String

    public init(name: String, startFolder: String, gitName: String, gitEmail: String) {
        (self.name, self.startFolder, self.gitName, self.gitEmail) = (name, startFolder, gitName, gitEmail)
    }
}

public struct PanelRecords: Sendable, Equatable {
    public var agents: [AgentRecord]
    public var imports: [ImportRecord]
    public var profile: ProfileRecord

    public init(agents: [AgentRecord], imports: [ImportRecord], profile: ProfileRecord) {
        (self.agents, self.imports, self.profile) = (agents, imports, profile)
    }

    /// Sample records for previews and tests, until the FFI delivers real ones.
    public static let sample = PanelRecords(
        agents: [
            AgentRecord(id: "claude", name: "Claude Code", detail: "~/.claude · status hooks installed", state: .ready, hooks: .remove),
            AgentRecord(id: "codex", name: "Codex", detail: "~/.codex · hooks need repair", state: .needsRepair, hooks: .repair),
            AgentRecord(id: "gemini", name: "Gemini CLI", detail: "Found on PATH · not set up", state: .statusFromOutput, hooks: .install),
            AgentRecord(id: "aider", name: "Aider", detail: "Not on PATH", state: .notInstalled),
        ],
        imports: [
            ImportRecord(id: "ghostty", app: "Ghostty", summary: "Font, theme, 14 shortcuts"),
            ImportRecord(id: "iterm", app: "iTerm2", summary: "Font, colours, scrollback"),
        ],
        profile: ProfileRecord(name: "Default", startFolder: "~", gitName: "", gitEmail: ""))
}

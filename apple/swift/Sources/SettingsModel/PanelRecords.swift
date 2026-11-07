// The records the list-like pages draw (agents and their hooks, terminals found to import from, the profile).
// They come from the host (`SettingsHost`), since each reads the disk or the app's state in Rust; the pages show
// them and send the buttons back as actions.

public struct AgentRecord: Sendable, Equatable, Identifiable {
    public enum State: Sendable { case ready, needsRepair, notInstalled }
    public let id: String
    public let name: String
    public let detail: String
    public let state: State

    public init(id: String, name: String, detail: String, state: State) {
        (self.id, self.name, self.detail, self.state) = (id, name, detail, state)
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
            AgentRecord(id: "claude", name: "Claude Code", detail: "~/.claude · status hooks installed", state: .ready),
            AgentRecord(id: "codex", name: "Codex", detail: "~/.codex · hooks need repair", state: .needsRepair),
            AgentRecord(id: "gemini", name: "Gemini CLI", detail: "Found on PATH · not set up", state: .notInstalled),
        ],
        imports: [
            ImportRecord(id: "ghostty", app: "Ghostty", summary: "Font, theme, 14 shortcuts"),
            ImportRecord(id: "iterm", app: "iTerm2", summary: "Font, colours, scrollback"),
        ],
        profile: ProfileRecord(name: "Default", startFolder: "~", gitName: "", gitEmail: ""))
}

// The Agents page's skill rows and the Feature flags page's switches, as the core writes them.

/// One agent's Wormhole skill (the folder that teaches it to drive Wormhole through `wh`).
public struct SkillRecord: Sendable, Equatable, Identifiable {
    /// `Covered`: not installed here, but a folder the agent also reads has it. `unreadable`: see `problem`.
    public enum State: Sendable { case notInstalled, current, outdated, foreign, covered, unreadable }
    /// The target's word (`claude`).
    public let id: String
    public let name: String
    public let state: State
    /// "Skill installed in" and its kin.
    public let detail: String
    /// The skill's folder as a person says it, or the covering one.
    public let shown: String
    public let problem: String?

    public init(id: String, name: String, state: State, detail: String, shown: String, problem: String? = nil) {
        (self.id, self.name, self.state, self.detail, self.shown, self.problem) = (id, name, state, detail, shown, problem)
    }
}

/// A switch of the Feature flags page.
public struct FlagRecord: Sendable, Equatable, Identifiable {
    public var id: String { key }
    public let key: String
    public let title: String
    public let description: String
    public let on: Bool
    /// What the flag is with no entry in `config.json`: the row is changed when `on` differs.
    public let defaultOn: Bool

    public init(key: String, title: String, description: String, on: Bool, defaultOn: Bool) {
        (self.key, self.title, self.description, self.on, self.defaultOn) = (key, title, description, on, defaultOn)
    }
}

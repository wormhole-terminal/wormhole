// The project at a terminal's folder as the Project tab draws it (`features/project`, from core's `LoadedProject`):
// its trust, what it would run, the switches, the actions and prompts, the layout and anything that could not load.
// The backend builds it from the files; every line of wording that depends on core's rules (the hint under a
// switch) arrives already written.

public enum TrustStatus: Sendable {
    case untrusted, trusted
    /// Was trusted, but the files changed since: approved again before use.
    case changed

    public var badge: String {
        switch self {
        case .trusted: "trusted"
        case .changed: "changed"
        case .untrusted: "not trusted"
        }
    }
}

/// One line of what a project wants to run, with the parts kept apart so the notice sets what runs in its own
/// colour: "action test (key ⌃⌘T): run `cargo test` (current)".
public struct TrustLine: Hashable, Sendable {
    public var what: String
    public var runs: String
    public var note: String

    public init(what: String, runs: String = "", note: String = "") {
        (self.what, self.runs, self.note) = (what, runs, note)
    }
}

public struct FeatureRow: Hashable, Sendable, Identifiable {
    public var feature: ProjectFeature
    public var on: Bool
    /// The line under the switch, and whether it is a warning (held back by trust).
    public var hint: String
    public var warns: Bool

    public var id: ProjectFeature { feature }

    public init(_ feature: ProjectFeature, on: Bool, hint: String, warns: Bool = false) {
        (self.feature, self.on, self.hint, self.warns) = (feature, on, hint, warns)
    }
}

public struct ProjectAction: Hashable, Sendable, Identifiable {
    public var id: String
    public var title: String
    /// Its description, or else what it does.
    public var detail: String?
    /// The key it is bound to, as glyphs ("⌃⌘T").
    public var key: String?

    public init(id: String, title: String, detail: String? = nil, key: String? = nil) {
        (self.id, self.title, self.detail, self.key) = (id, title, detail, key)
    }
}

public struct ProjectPrompt: Hashable, Sendable, Identifiable {
    public var id: String
    public var title: String
    public var detail: String?

    public init(id: String, title: String, detail: String? = nil) {
        (self.id, self.title, self.detail) = (id, title, detail)
    }
}

public struct ProjectState: Equatable, Sendable {
    /// The project's root folder, and its name for the head ("wormhole/.wormhole").
    public var root: String
    public var status: TrustStatus
    /// What approving would run; empty when only settings or the like are held back.
    public var commands: [TrustLine]
    /// "its settings in config.json and actions in actions.json": what trusting applies.
    public var applies: String
    /// Whether the notice is shown at all: something here can run commands or waits for trust.
    public var asksForTrust: Bool
    public var approving = false
    public var features: [FeatureRow]
    public var actions: [ProjectAction]
    public var prompts: [ProjectPrompt]
    /// The names of the terminals the layout opens.
    public var layout: [String]
    public var errors: [String]
    public var warnings: [String]

    public init(
        root: String, status: TrustStatus, commands: [TrustLine] = [], applies: String = "", asksForTrust: Bool = false,
        features: [FeatureRow] = [], actions: [ProjectAction] = [], prompts: [ProjectPrompt] = [], layout: [String] = [],
        errors: [String] = [], warnings: [String] = []
    ) {
        (self.root, self.status, self.commands, self.applies, self.asksForTrust) = (root, status, commands, applies, asksForTrust)
        (self.features, self.actions, self.prompts, self.layout, self.errors, self.warnings) = (features, actions, prompts, layout, errors, warnings)
    }

    public var name: String { root.split(separator: "/").last.map(String.init) ?? root }

    /// The notice's title and text for a project that isn't trusted: what it wants to run, or with no command to
    /// list, what trusting applies.
    public var notice: (title: String, text: String) {
        let changed = status == .changed
        let title = changed ? "Files changed since you trusted this project" : "Trust this project?"
        switch (changed, !commands.isEmpty) {
        case (true, true): return (title, "Review what it would run now and approve again.")
        case (true, false): return (title, "Trusting it applies \(applies). Review it and approve again.")
        case (false, true): return (title, "It wants to run these commands. Nothing runs until you approve.")
        case (false, false): return (title, "Trusting it applies \(applies). Nothing applies until you approve.")
        }
    }

    /// The quiet line of a trusted project.
    public var trustedLine: String {
        commands.isEmpty ? "Applies \(applies)." : "Actions, the terminal layout and project env may run."
    }

    /// How many lines the notice shows in full; a longer list shows `shownLines` and "View all N…".
    public static let linesMax = 6
    public static let linesShown = 5
}

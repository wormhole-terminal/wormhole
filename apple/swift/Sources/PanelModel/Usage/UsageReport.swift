// What an agent last reported of its plan's limits (`wormhole_core::agents::usage`): how much of each limit window
// is used and when it starts over. Times are Unix seconds and the caller passes `now`, so nothing here reads a clock.

public struct PlanLimit: Hashable, Sendable {
    public var label: String
    /// 0 to 100.
    public var used: Double
    public var resetsAt: Int?

    public init(label: String, used: Double, resetsAt: Int? = nil) {
        self.label = label
        self.used = used
        self.resetsAt = resetsAt
    }

    /// A gauge's level: the colour its bar is drawn in. From 80 % on it warns; at the limit it is danger.
    public enum Level: Sendable { case normal, warn, danger }

    /// The share used (0...1) and its words, "25% · resets 34m". A window that has reset since, which the numbers
    /// can't say anything of, reads "Awaiting update".
    public func shown(now: Int) -> (share: Double, words: String) {
        if let resetsAt, resetsAt <= now { return (0, "Awaiting update") }
        let clamped = min(max(used, 0), 100)
        var words = "\(Int(clamped.rounded()))%"
        if let resetsAt { words += " \u{b7} resets \(UsageText.duration(resetsAt - now))" }
        return (clamped / 100, words)
    }

    public static func level(of share: Double) -> Level { share >= 1 ? .danger : share >= 0.8 ? .warn : .normal }
}

/// An agent's bundled logo: the SVG's path data in its 24 point box (the core's registry holds it; nothing here is a brand colour,
/// the panel draws it in its muted one).
public struct AgentLogoShape: Hashable, Sendable {
    public var paths: [String]
    public var evenOdd: Bool

    public init(paths: [String], evenOdd: Bool) { (self.paths, self.evenOdd) = (paths, evenOdd) }
}

public struct UsageReport: Identifiable, Hashable, Sendable {
    public var agent: String
    /// The card's name for the agent ("Claude") and the letters that stand for it where it has no logo, from the registry.
    public var name: String
    public var monogram: String
    public var logo: AgentLogoShape?
    public var limits: [PlanLimit]
    /// Gemini reports the latest response's tokens instead of account limits.
    public var responseTokens: Int?
    public var updated: Int

    public var id: String { agent }

    public init(
        agent: String, name: String? = nil, monogram: String? = nil, logo: AgentLogoShape? = nil, limits: [PlanLimit] = [],
        responseTokens: Int? = nil, updated: Int
    ) {
        self.agent = agent
        self.name = name ?? agent.prefix(1).uppercased() + agent.dropFirst()
        self.monogram = monogram ?? String(agent.prefix(2)).uppercased()
        self.logo = logo
        self.limits = limits
        self.responseTokens = responseTokens
        self.updated = updated
    }
}

public enum UsageText {
    /// "45s", "34m", "2h 5m", "3d": the short form the limits use.
    public static func duration(_ seconds: Int) -> String {
        let secs = max(seconds, 0)
        switch secs {
        case ..<60: return "\(secs)s"
        case ..<3600: return "\(secs / 60)m"
        case ..<86_400:
            let minutes = secs % 3600 / 60
            return minutes == 0 ? "\(secs / 3600)h" : "\(secs / 3600)h \(minutes)m"
        // To the nearest day: three days less a minute still reads "3d".
        default: return "\((secs + 43_200) / 86_400)d"
        }
    }

    /// When the numbers came: "Updated just now", "Updated 3m ago".
    public static func updated(_ updated: Int, now: Int) -> String {
        now - updated < 60 ? "Updated just now" : "Updated \(duration(now - updated)) ago"
    }
}

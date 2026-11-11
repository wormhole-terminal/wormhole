// Backends that answer from fixed data, for previews, tests and the app until the Rust FFI supplies the real
// ones: each is a small class behind the protocol the window draws from, never a second implementation of a rule.

import Foundation

@MainActor
public final class SampleOnboardingBackend: OnboardingBackend {
    public let preferences = OnboardingPreferences()
    public var trustCard: TrustCard? = TrustCard(name: "terminal", directory: ".wormhole", commands: 2)
    public let settingsKeys = "\u{2318},"
    public var agents: [FoundAgent]
    public var others: Int
    public private(set) var finished: (folder: URL?, firstRun: Bool)?
    public private(set) var trusted: [URL] = []

    public init(agents: [FoundAgent] = SampleOnboardingBackend.someAgents, others: Int = 6) {
        (self.agents, self.others) = (agents, others)
    }

    public static let someAgents = [
        FoundAgent(id: "claude", name: "Claude Code", hooks: .installed, path: "~/.claude/settings.json"),
        FoundAgent(id: "codex", name: "Codex", hooks: .notInstalled, path: "~/.codex/hooks.json"),
        FoundAgent(id: "aider", name: "Aider", hooks: .unsupported),
    ]

    public func scanAgents() async -> (found: [FoundAgent], others: Int) { (agents, others) }

    public func installHooks(agent: String) async throws -> FoundAgent {
        guard let index = agents.firstIndex(where: { $0.id == agent }) else { throw CocoaError(.fileNoSuchFile) }
        agents[index].hooks = .installed
        return agents[index]
    }

    public func trust(_ folder: URL) { trusted.append(folder) }

    public func places() -> [Place] {
        Place.list(
            home: URL(fileURLWithPath: "/Users/you"), recents: [URL(fileURLWithPath: "/Users/you/Projects/terminal")],
            exists: { _ in true })
    }

    public func chooseFolder() async -> URL? { URL(fileURLWithPath: "/Users/you/Projects/app") }
    public func finish(folder: URL?, firstRun: Bool) { finished = (folder, firstRun) }
}

// A `UsageBackend` over fixed reports.

public struct MockUsageBackend: UsageBackend {
    public var fixed: [UsageReport]

    public init(_ reports: [UsageReport] = PanelSamples.usage) { fixed = reports }

    public func reports() async -> [UsageReport] { fixed }
    public func changes() -> AsyncStream<Void> { AsyncStream { $0.finish() } }
}

extension PanelSamples {
    public static let usage = [
        UsageReport(
            agent: "claude",
            limits: [
                PlanLimit(label: "5-hour limit", used: 25, resetsAt: now + 34 * 60),
                PlanLimit(label: "Weekly \u{b7} all models", used: 86, resetsAt: now + 2 * 86_400 + 3600),
                PlanLimit(label: "Weekly \u{b7} Opus", used: 100, resetsAt: now + 3 * 86_400),
            ], updated: now - 90),
        UsageReport(agent: "codex", limits: [PlanLimit(label: "Weekly limit", used: 8, resetsAt: now - 60)], updated: now - 3600),
        UsageReport(agent: "gemini", responseTokens: 18_412, updated: now - 20),
    ]
}

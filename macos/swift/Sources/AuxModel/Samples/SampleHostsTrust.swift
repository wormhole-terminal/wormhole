// Sample backends for the Session Hosts and project trust windows.

import Foundation

@MainActor
public final class SampleHostsBackend: HostsBackend {
    public var current = HostsSamples.report
    public private(set) var ended: [String] = []
    public private(set) var forgotten: [String] = []

    public init() {}

    public func report() async -> HostsReport { current }
    public func changes() -> AsyncStream<Void> { AsyncStream { _ in } }

    public func end(_ id: String) async {
        ended.append(id)
        current.cards.removeAll { $0.id == id }
    }

    public func forget(_ id: String) async {
        forgotten.append(id)
        current.cards.removeAll { $0.id == id }
    }
}

@MainActor
public final class SampleTrustBackend: TrustBackend {
    public var current: TrustReview
    public let biometricsAvailable = true
    public private(set) var approvals = 0

    public init(status: TrustStatus = .untrusted) {
        current = TrustReview(
            name: "terminal", directory: ".wormhole", status: status,
            lines: [
                TrustLine(what: "action test (key \u{2303}\u{2318}T): run", runs: "cargo test", note: "(current)"),
                TrustLine(what: "action lint: run", runs: "cargo clippy --workspace -- -D warnings"),
                TrustLine(what: "layout: starts", runs: "npm run dev", note: "in a second terminal"),
                TrustLine(what: "env: sets", runs: "RUST_LOG=debug"),
            ])
    }

    public func review() -> TrustReview { current }
    public func changes() -> AsyncStream<Void> { AsyncStream { _ in } }

    public func approve() async {
        approvals += 1
        current.status = .trusted
    }
}

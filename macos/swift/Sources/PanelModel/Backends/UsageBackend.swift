// Where the Usage tab's reports come from: what the agents' status lines and hooks forwarded to the app
// (`wormhole_core::agents::usage`). One value for every window.

public protocol UsageBackend: Sendable {
    func reports() async -> [UsageReport]
    /// Fires when an agent reported anew.
    func changes() -> AsyncStream<Void>
}

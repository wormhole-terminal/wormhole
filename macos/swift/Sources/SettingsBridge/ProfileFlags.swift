// Which feature flags are on in a profile's own settings, whichever profile Wormhole is in: a window reads its own
// profile's switches, never the shown one's (a window opened in, or moved to, another profile). A handle per profile is
// kept, so a flag changed in that profile's `config.json` is seen (the handle follows the file).

import Foundation
import WormholeBindings

/// Hears a profile's handle wake, which Rust calls on a thread of its own, and passes it on.
private final class Relay: SettingsListener {
    private let woke: @Sendable () -> Void

    init(_ woke: @escaping @Sendable () -> Void) { self.woke = woke }

    func wake() { woke() }
}

@MainActor
public final class ProfileFlags {
    private var handles: [String: SettingsHandle] = [:]
    /// Told when a profile's `config.json` was read again: its switches may have moved (the windows of that profile take
    /// them, the menus are built again).
    public var changed: (@MainActor (String) -> Void)?

    public init() {}

    /// The keys of the features that are off in `profile` (the core's feature mask: the switch and the gate).
    public func featuresOff(profile: String) -> Set<String> {
        Set(handle(profile)?.featuresOff() ?? [])
    }

    /// The keys of the flags that are on in `profile`; none where its settings cannot be read.
    public func on(profile: String) -> Set<String> {
        Set(handle(profile)?.featureFlags().filter(\.on).map(\.key) ?? [])
    }

    private func handle(_ profile: String) -> SettingsHandle? {
        if handles[profile] == nil {
            let relay = Relay { [weak self] in
                DispatchQueue.main.async { MainActor.assumeIsolated { self?.woke(profile) } }
            }
            handles[profile] = try? SettingsHandle.open(profile: profile, listener: relay)
        }
        return handles[profile]
    }

    private func woke(_ profile: String) {
        _ = handles[profile]?.drain()
        changed?(profile)
    }
}

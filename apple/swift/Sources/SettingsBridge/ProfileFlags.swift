// Which feature flags are on in a profile's own settings, whichever profile Wormhole is in: a window reads its own
// profile's switches, never the shown one's (a window opened in, or moved to, another profile). A handle per profile is
// kept, so a flag changed in that profile's `config.json` is seen (the handle follows the file).

import Foundation
import WormholeBindings

private final class Silent: SettingsListener {
    func wake() {}
}

@MainActor
public final class ProfileFlags {
    private var handles: [String: SettingsHandle] = [:]

    public init() {}

    /// The keys of the flags that are on in `profile`; none where its settings cannot be read.
    public func on(profile: String) -> Set<String> {
        if handles[profile] == nil { handles[profile] = try? SettingsHandle.open(profile: profile, listener: Silent()) }
        return Set(handles[profile]?.featureFlags().filter(\.on).map(\.key) ?? [])
    }
}

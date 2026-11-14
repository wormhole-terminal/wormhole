// Agent access of a parked profile: one Wormhole left, whose windows are out of sight with their terminals running.
// Its programs can still ask (the hook server judges a request by the profile of the window it reaches, and a question
// waits there for a window), so a change of its mode must settle what waits and what is open, as the GPUI app's tick does
// for every profile that has a window (`agent_access/ask.rs` `tick`, `settle_mode`). The shown profile is followed by
// `SettingsApplier`, a profile with an odd window in sight by its `ProfileLook`; a parked one only by this: its
// `config.json`, followed by the core's watch, and the four keys the server reads.

import Foundation
import SettingsModel

#if WITH_UNIFFI
    import SettingsBridge
#endif

@MainActor
final class ParkedAccess {
    /// The keys the hook server reads again (`SettingsApplier`, `ProfileLook`: the same four).
    static let keys = ["agent_access", "control_write", "feature_flags", "never_hide_lines"]

    /// The profiles that need a follower of this kind: every profile with a window (in sight or parked) but the shown one,
    /// which `SettingsApplier` follows, and those with a look of their own, which follows them. So each profile with
    /// terminals has exactly one follower, whatever order its windows came and went in.
    static func wanted(windows: [String], shown: String, looks: Set<String>) -> Set<String> {
        Set(windows).subtracting(looks).subtracting([shown])
    }

    let profile: String
    private let backend: any SettingsBackend
    private var applied: [JSONValue?]
    private var observation: SettingsObservation?

    /// Follows `backend` (profile `profile`'s settings) and calls `settle` with the profile when one of the keys changed.
    init(profile: String, backend: any SettingsBackend, settle: @escaping @MainActor (String) -> Void) {
        (self.profile, self.backend) = (profile, backend)
        applied = Self.keys.map { backend.values[$0] }
        observation = backend.observe { [weak self] in
            guard let self else { return }
            let now = Self.keys.map { self.backend.values[$0] }
            guard now != applied else { return }
            applied = now
            settle(profile)
        }
    }
}

extension AppController {
    /// Follows the agent access of every profile with windows that nothing else follows, and stops following the ones that
    /// came back or went. Called whenever the set can change: a profile switch, a profile ended, a profile's look made or
    /// dropped (a window of it opened or closed beside the shown ones: `profileLooks`' `didSet`).
    func followParkedAccess() {
        let parked = ParkedAccess.wanted(
            windows: sessions.map(\.windows.profile), shown: windowsHost.profile, looks: Set(profileLooks.keys))
        for id in parkedAccess.keys where !parked.contains(id) { parkedAccess[id] = nil }
        #if WITH_UNIFFI
            guard options.isReal else { return }
            for id in parked where parkedAccess[id] == nil {
                guard let kit = try? SettingsBridge.make(profile: id) else { continue }
                parkedAccess[id] = ParkedAccess(profile: id, backend: kit.backend) { [weak self] in self?.hooks?.settingsChanged(profile: $0) }
            }
        #endif
    }
}

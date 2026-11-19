import Foundation
import SettingsModel
import Testing

@testable import Wormhole

/// A parked profile's agent access settles when its file changes (`agent_access/ask.rs` `tick` settles every profile with
/// a window): a change of the mode reaches the hook server once, with that profile, and a change of anything else does not.
@MainActor
struct ParkedAccessTests {
    @Test func aChangeOfTheModeSettlesTheParkedProfileOnce() {
        let backend = MockSettingsBackend()
        var settled: [String] = []
        let follower = ParkedAccess(profile: "work", backend: backend) { settled.append($0) }
        backend.set("font_size", to: .number(15))
        #expect(settled.isEmpty, "a key the server does not read settles nothing")
        backend.set("agent_access", to: .string("off"))
        #expect(settled == ["work"])
        backend.set("tab_width", to: .number(2))
        #expect(settled == ["work"], "the mode did not move again")
        _ = follower
    }

    /// The order the settings audit found half-wired: "work" parked; a window of it opened while "default" is shown (its own
    /// look follows it then); a switch to "home"; that window closed. Each step does what `followParkedAccess` does (run
    /// after every switch and whenever a look is made or dropped): after each, every profile with windows but the shown one
    /// has exactly one follower, a look or a parked one, and a change of "work"'s mode at the end still settles, once.
    @Test func everyProfileWithWindowsButTheShownOneHasExactlyOneFollower() {
        let files = ["default", "work", "home"].reduce(into: [String: MockSettingsBackend]()) { $0[$1] = MockSettingsBackend() }
        var settled: [String] = []
        var parked: [String: ParkedAccess] = [:]
        func follow(windows: [String], shown: String, looks: Set<String>) {
            let wanted = ParkedAccess.wanted(windows: windows, shown: shown, looks: looks)
            for id in parked.keys where !wanted.contains(id) { parked[id] = nil }
            for id in wanted where parked[id] == nil {
                if let file = files[id] { parked[id] = ParkedAccess(profile: id, backend: file) { settled.append($0) } }
            }
            for id in Set(windows).subtracting([shown]) {
                let followers = (looks.contains(id) ? 1 : 0) + (parked[id] == nil ? 0 : 1)
                #expect(followers == 1, "\(id) has \(followers) followers")
            }
            #expect(parked[shown] == nil && !looks.contains(shown), "the shown profile is the app's own to follow")
        }
        // "work" was shown and is left: its window is parked.
        follow(windows: ["work", "default"], shown: "default", looks: [])
        // A window of "work" opened beside the shown ones: its look follows "work" now.
        follow(windows: ["work", "work", "default"], shown: "default", looks: ["work"])
        // A switch to "home": "default"'s window is parked too.
        follow(windows: ["work", "work", "default", "home"], shown: "home", looks: ["work"])
        // The window of "work" in sight is closed: its look goes, and its parked window still has terminals.
        follow(windows: ["work", "default", "home"], shown: "home", looks: [])
        files["work"]?.set("agent_access", to: .string("off"))
        #expect(settled == ["work"], "the parked profile's mode still settles, once")
    }
}

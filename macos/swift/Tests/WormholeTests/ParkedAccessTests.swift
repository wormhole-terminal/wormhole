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
}

import Testing

@testable import Wormhole

/// Which window's project goes over the settings of the profile Wormhole is in: only a window of that profile, the one
/// used last (`features/project/detect.rs:83-89`: "a project another profile trusts does not change the look and
/// behaviour of this one").
struct ProjectSettingsSourceTests {
    struct Window: Equatable {
        var name: String
        var profile: String
        var inSight = true
    }

    private static func read(front: Window?, before: Window?, shown: String) -> String? {
        ProjectSettingsSource.window(front: front, readBefore: before, shown: shown) { $0.inSight ? $0.profile : nil }?.name
    }

    @Test func theWindowInFrontIsReadWhenItIsOfTheShownProfile() {
        let front = Window(name: "a", profile: "default"), before = Window(name: "b", profile: "default")
        #expect(Self.read(front: front, before: before, shown: "default") == "a")
        #expect(Self.read(front: front, before: nil, shown: "default") == "a")
    }

    @Test func aWindowOfAnotherProfileInFrontIsNeverRead() {
        let odd = Window(name: "odd", profile: "work"), own = Window(name: "own", profile: "default")
        #expect(Self.read(front: odd, before: own, shown: "default") == "own", "the window read before goes on being read")
        #expect(Self.read(front: odd, before: nil, shown: "default") == nil, "and with none, no project is laid on")
        #expect(Self.read(front: odd, before: odd, shown: "default") == nil)
    }

    @Test func afterASwitchTheWindowsOfTheProfileLeftAreNotRead() {
        // Wormhole went from "default" to "work": the window read before is parked, and would be of another profile anyway.
        let left = Window(name: "left", profile: "default", inSight: false), back = Window(name: "back", profile: "work")
        #expect(Self.read(front: back, before: left, shown: "work") == "back")
        #expect(Self.read(front: nil, before: left, shown: "work") == nil)
        // A window moved to another profile is dropped where it was read, and read where it came into.
        let moved = Window(name: "moved", profile: "work")
        #expect(Self.read(front: moved, before: moved, shown: "default") == nil)
        #expect(Self.read(front: moved, before: moved, shown: "work") == "moved")
    }

    @Test func aClosedOrParkedWindowIsNotRead() {
        let gone = Window(name: "gone", profile: "default", inSight: false)
        #expect(Self.read(front: gone, before: gone, shown: "default") == nil)
    }
}

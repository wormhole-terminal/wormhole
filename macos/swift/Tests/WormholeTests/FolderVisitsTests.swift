import Testing

@testable import Wormhole

/// The folder picker's recent folders (`app_state/recent_folders.rs`): every folder a terminal is in counts from the first
/// look at the windows, which the app takes when its first window opens, not when the picker is first used.
struct FolderVisitsTests {
    private typealias Window = FolderVisits.Window
    private typealias Visit = FolderVisits.Outcome.Visit

    @Test func aTerminalsFolderIsAVisitTheFirstTimeItIsSeenAndEachTimeItChanges() {
        var visits = FolderVisits()
        let first = visits.take([Window(profile: "default", recorded: true, folders: [(1, "~/code"), (2, nil)])])
        #expect(first.visits == [Visit(profile: "default", folder: "~/code")], "the folder a restored terminal starts in counts")
        #expect(visits.take([Window(profile: "default", recorded: true, folders: [(1, "~/code"), (2, nil)])]).visits.isEmpty)
        let moved = visits.take([Window(profile: "default", recorded: true, folders: [(1, "~/code/src"), (2, "~/notes")])])
        #expect(moved.visits == [Visit(profile: "default", folder: "~/code/src"), Visit(profile: "default", folder: "~/notes")])
    }

    @Test func aWindowOpenedLaterIsCountedFromItsFirstFolderInItsOwnProfile() {
        var visits = FolderVisits()
        _ = visits.take([Window(profile: "default", recorded: true, folders: [(1, "~/code")])])
        let both = visits.take([
            Window(profile: "default", recorded: true, folders: [(1, "~/code")]),
            Window(profile: "work", recorded: true, folders: [(7, "~/work")]),
        ])
        #expect(both.visits == [Visit(profile: "work", folder: "~/work")])
    }

    @Test func withTheFlagOffNothingIsRecordedAndTheListIsForgottenOnce() {
        var visits = FolderVisits()
        let off = [Window(profile: "default", recorded: false, folders: [(1, "~/code")])]
        #expect(visits.take(off) == FolderVisits.Outcome(visits: [], forget: ["default"]))
        #expect(visits.take(off) == FolderVisits.Outcome(), "forgotten once, not at every look")
        // A profile with no terminal in a folder yet has nothing to forget.
        #expect(visits.take([Window(profile: "work", recorded: false, folders: [(7, nil)])]).forget.isEmpty)
        // Back on: the folder the terminal is in counts again.
        let on = visits.take([Window(profile: "default", recorded: true, folders: [(1, "~/code")])])
        #expect(on.visits == [Visit(profile: "default", folder: "~/code")])
    }
}

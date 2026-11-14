import Testing

@testable import TerminalCore

/// A terminal nobody sees is drained at once for a lone event and once per interval under a flood (`MetalTerminalView`
/// asks this for a minimised window's view and for one in no window). Order guarded: the source wakes, is drained and
/// so re-armed, and the program's next read burst wakes it again before the interval is over.
@Suite struct DrainPaceTests {
    @Test func aLoneWakeIsDrainedAtOnce() {
        var pace = DrainPace()
        #expect(pace.woke(), "a bell, a title or an exit is heard without waiting")
        #expect(!pace.elapsed(), "nothing came meanwhile: no drain, and no timer left running")
        #expect(pace.woke(), "the next lone wake is at once again")
    }

    @Test func aFloodIsOneDrainPerInterval() {
        var pace = DrainPace()
        #expect(pace.woke())
        // The re-armed source wakes again and again within the interval: none of these drains.
        for _ in 0..<1000 { #expect(!pace.woke()) }
        #expect(pace.elapsed(), "one drain for all of them, at the interval's end")
        #expect(!pace.woke(), "and the interval runs again")
        #expect(pace.elapsed())
        #expect(!pace.elapsed(), "the flood ended: the pace stops")
    }
}

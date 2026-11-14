import Testing

@testable import TerminalCore

/// A terminal nobody sees is drained at once for a lone event and once per interval under a flood (`MetalTerminalView`
/// asks this for a minimised window's view and for one in no window). Order guarded: the source wakes, is drained and
/// so re-armed, and the program's next read burst wakes it again before the interval is over.
@Suite struct DrainPaceTests {
    // `#expect` takes its expression as a value, so a mutating call is made first and its answer expected.
    @Test func aLoneWakeIsDrainedAtOnce() {
        var pace = DrainPace()
        let first = pace.woke()
        #expect(first, "a bell, a title or an exit is heard without waiting")
        let ended = pace.elapsed()
        #expect(!ended, "nothing came meanwhile: no drain, and no timer left running")
        let next = pace.woke()
        #expect(next, "the next lone wake is at once again")
    }

    @Test func aFloodIsOneDrainPerInterval() {
        var pace = DrainPace()
        let first = pace.woke()
        #expect(first)
        // The re-armed source wakes again and again within the interval: none of these drains.
        var drained = 0
        for _ in 0..<1000 where pace.woke() { drained += 1 }
        #expect(drained == 0)
        let atEnd = pace.elapsed()
        #expect(atEnd, "one drain for all of them, at the interval's end")
        let during = pace.woke()
        #expect(!during, "and the interval runs again")
        let second = pace.elapsed()
        #expect(second)
        let last = pace.elapsed()
        #expect(!last, "the flood ended: the pace stops")
    }
}

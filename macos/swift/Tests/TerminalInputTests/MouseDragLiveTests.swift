#if WITH_UNIFFI
import AppKit
import Testing
import TerminalCore
import UniFFISource

@testable import TerminalMetal

/// A drag in button-event tracking (1002), where the program is told of the motion once for each cell the pointer comes to.
@MainActor
@Suite(.serialized) struct MouseDragLiveTests {
    @Test func buttonEventModeReportsADragOnceForEachNewCell() async throws {
        let bench = try ByteBench(#"\033[?1002h\033[?1006h"#)
        #expect(await bench.start())
        bench.press(.left, row: 3, column: 2)
        bench.drag(to: 3, 2)
        bench.drag(to: 3, 3)
        bench.drag(to: 3, 3)
        bench.drag(to: 4, 3)
        bench.release(.left, row: 4, column: 3)
        // 32 is motion with the left button held; a move inside a cell and the same cell again are not new cells.
        await bench.expect(ByteBench.hex("\u{1b}[<0;3;4M\u{1b}[<32;4;4M\u{1b}[<32;4;5M\u{1b}[<0;4;5m"), "press, a motion for each new cell (not for the cell the press was in, nor the same cell twice), release")
        // Nothing for a hover without a button at this level.
        bench.move(to: 6, 6)
        await bench.expect("")
    }
}
#endif

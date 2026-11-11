import Foundation
import Testing
import TerminalCore

@testable import TerminalMetal

/// The frame's work guards, as GPUI's counting tests (`performance.md`): what applying a frame costs in rows rebuilt,
/// words shaped and main-thread allocations, held to budgets so a change that makes every frame do more fails here.
/// `apple/scripts/perf-guards.sh` runs this suite with the rest of `TerminalCoreTests`.
@MainActor
@Suite struct FrameWorkGuardTests {
    private let size = GridSize(cols: 80, rows: 24)

    /// A frame with `rows` of a sentence, each row's words the same, so the second application only hits the word cache.
    private func frame(changing rows: Range<Int>) -> Data {
        var writer = FrameWriter(
            header: FrameHeader(cols: size.cols, rows: size.rows, changedRows: rows.count, isFull: rows.count == size.rows, cursor: nil),
            rowCapacity: rows.count)
        let text = Array("cargo test --workspace passed in 12.3s, all good ".unicodeScalars)
        let cells = (0..<size.cols).map { PackedCell(scalar: text[$0 % text.count].value) }
        for row in rows { cells.withUnsafeBufferPointer { writer.appendRow(index: row, size: .single, cells: $0) } }
        return writer.bytes
    }

    private func warmed() throws -> MetalRenderer {
        let renderer = try MetalRenderer(configuration: RendererConfiguration(font: .system(family: "Menlo"), pointSize: 13, scale: 2))
        _ = try renderer.apply(frame: frame(changing: 0..<size.rows))
        _ = try renderer.apply(frame: frame(changing: 0..<size.rows))
        return renderer
    }

    @Test func aFrameThatChangesNothingRebuildsNothingAndBarelyAllocates() throws {
        let renderer = try warmed()
        let before = renderer.counters
        let (rebuilt, allocations) = try AllocationCounter.counting { try renderer.apply(frame: frame(changing: 0..<0)) }
        #expect(rebuilt == 0)
        #expect((renderer.counters - before).wordsShaped == 0)
        #expect(allocations <= Budget.emptyFrame, "an empty frame allocated \(allocations) times")
    }

    /// What a still screen's frame is now (`prepare_frame` says 0): nothing to parse, nothing allocated.
    @Test func anEmptyFrameAllocatesNothing() throws {
        let renderer = try warmed()
        let (rebuilt, allocations) = try AllocationCounter.counting { try renderer.apply(frame: Data()) }
        #expect(rebuilt == 0 && allocations == 0, "an empty frame allocated \(allocations) times")
    }

    @Test func aChangedRowCostsItsRowAndItsCachedWordsOnly() throws {
        let renderer = try warmed()
        let before = renderer.counters
        let (rebuilt, allocations) = try AllocationCounter.counting { try renderer.apply(frame: frame(changing: 0..<size.rows)) }
        #expect(rebuilt == size.rows)
        #expect((renderer.counters - before).wordsShaped == 0, "the same words again come from the cache")
        #expect(allocations <= Budget.perRow * size.rows, "\(size.rows) rows allocated \(allocations) times")
    }

    @Test func theCounterSeesAllocationsOnTheMainThread() {
        let (boxes, allocations) = AllocationCounter.counting { (0..<100).map { NSNumber(value: Double($0) + 0.5) } }
        #expect(boxes.count == 100)
        #expect(allocations >= 100)
    }

    /// Measured on this tree (2026-11-11, round S wave 2) in the debug build `swift test` makes: 133 for a frame of a header only
    /// and 238 a row (unoptimised code; the release app's `perf-profile` shows 0 for the fetch, apply and view of a still screen,
    /// whose frame is now empty, and 6 for the apply of 42 flooded rows). Lower them when a frame gets cheaper; raise them only with
    /// the reason in the commit.
    private enum Budget {
        static let emptyFrame = 140
        static let perRow = 250
    }
}

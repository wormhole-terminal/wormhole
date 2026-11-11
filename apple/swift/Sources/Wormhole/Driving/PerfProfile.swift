// `perf-profile [seconds] [name]`, after GPUI's: the selected terminal drawn again and again for that long, two
// milliseconds apart, and what each draw cost part by part (`FrameStats.swift` defines the spans) with the main
// thread's allocations per draw (`AllocationCounter`). The first three draws warm the caches and are left out, as
// `frames` does. Run it under a load (a program writing to the terminal) to see a frame's real work; on a still
// screen it measures a redraw of nothing new.

import Foundation
import TerminalCore
import TerminalMetal

@MainActor
enum PerfProfile {
    struct Report {
        let table: String
        let json: String
    }

    static func run(view: MetalTerminalView, seconds: Double) async -> Report {
        var samples: [FrameSample] = [], allocations: [Int] = []
        let countersAtStart = view.renderer.counters
        _ = view.renderer.gpuTimes.take()
        let end = MonotonicClock.now() + UInt64(max(0.1, seconds) * 1e9)
        while MonotonicClock.now() < end {
            let (sample, allocated) = AllocationCounter.counting { view.drawFrame(force: true) }
            samples.append(sample)
            allocations.append(allocated)
            // one-shot: the gap between two forced draws, as `frames` leaves one; the run ends at its deadline.
            try? await Task.sleep(for: .milliseconds(2))
        }
        // The last command buffers complete on Metal's threads a moment later.
        // one-shot: the GPU's last completion handlers.
        try? await Task.sleep(for: .milliseconds(150))
        let steady = Array(samples.dropFirst(min(3, samples.count))), allocated = Array(allocations.dropFirst(min(3, allocations.count)))
        let spans: [(String, [UInt64])] = [
            ("fetch", steady.map(\.fetchNs)), ("apply", steady.map(\.applyNs)), ("assemble", steady.map(\.assembleNs)),
            ("build", steady.map(\.buildNs)), ("encode", steady.map(\.encodeNs)), ("total", steady.map(\.totalNs)),
            ("wait", steady.map(\.waitNs)), ("cpu", steady.map(\.cpuNs)),
            ("gpu", view.renderer.gpuTimes.take().map { UInt64($0 * 1e9) }),
        ]
        let delta = view.renderer.counters - countersAtStart, grid = view.renderer.gridSize
        let perDraw = (rows: mean(steady.map(\.rowsRebuilt)), changed: mean(steady.map(\.changedRows)), allocations: mean(allocated))
        var table = "perf-profile draws=\(samples.count) seconds=\(seconds) grid=\(grid.cols)x\(grid.rows)\n"
        table += spans.map { "  \($0.0.padding(toLength: 9, withPad: " ", startingAt: 0)) \(Summary(nanoseconds: $0.1).line)" }.joined(separator: "\n")
        table += String(
            format: "\n  per draw: rows_rebuilt=%.1f changed_rows=%.1f allocations=%.1f (max %d)", perDraw.rows, perDraw.changed,
            perDraw.allocations, allocated.max() ?? 0)
        let parts = steady.map(\.allocations)
        table += String(
            format: "\n  allocations per draw by part: fetch=%.1f apply=%.1f view=%.1f draw=%.1f", mean(parts.map(\.fetch)),
            mean(parts.map(\.apply)), mean(parts.map(\.view)), mean(parts.map(\.draw)))
        table += "\n  run: words_shaped=\(delta.wordsShaped) glyphs_rasterised=\(delta.glyphsRasterised) atlas_resets=\(delta.atlasResets)"
        let json: [String: Any] = [
            "draws": samples.count, "seconds": seconds, "grid": "\(grid.cols)x\(grid.rows)",
            "spans_ms": Dictionary(uniqueKeysWithValues: spans.map { span in
                let s = Summary(nanoseconds: span.1)
                return (span.0, ["avg": s.mean, "p50": s.p50, "p99": s.p99, "max": s.max])
            }),
            "per_draw": ["rows_rebuilt": perDraw.rows, "changed_rows": perDraw.changed, "allocations": perDraw.allocations, "allocations_max": allocated.max() ?? 0],
            "run": ["words_shaped": delta.wordsShaped, "glyphs_rasterised": delta.glyphsRasterised, "atlas_resets": delta.atlasResets],
        ]
        let data = (try? JSONSerialization.data(withJSONObject: json, options: [.prettyPrinted, .sortedKeys])) ?? Data()
        return Report(table: table, json: String(decoding: data, as: UTF8.self))
    }

    private static func mean(_ values: [Int]) -> Double { values.isEmpty ? 0 : Double(values.reduce(0, +)) / Double(values.count) }
}

// `frames N`: N forced draws on the main thread, two milliseconds apart, the first three dropped, one line
// appended to frames.txt. The line begins exactly like GPUI's (`frames n=60 avg=..ms max=..ms first=..ms`) so
// `scripts/perf_tools.py` reads it; the Swift-only parts follow. `FrameStats.swift` says what each is.

import Foundation
import TerminalCore
import TerminalMetal

@MainActor
final class FramesRun {
    private let view: MetalTerminalView
    private let count: Int
    private let finished: @MainActor (String) -> Void
    private var samples: [FrameSample] = []
    private let countersAtStart: RendererCounters

    init(view: MetalTerminalView, count: Int, finished: @escaping @MainActor (String) -> Void) {
        self.view = view
        self.count = max(1, count)
        self.finished = finished
        countersAtStart = view.renderer.counters
    }

    func start() {
        _ = view.renderer.gpuTimes.take()
        step()
    }

    private func step() {
        guard samples.count < count else { return finish() }
        // A forced draw: it happens whether or not the frame changed anything, and waits for a free frame
        // instead of skipping (the wait is reported, not counted as work).
        samples.append(view.drawFrame(force: true))
        // Let the app breathe between draws, as GPUI's `frames` does (and as a display link would).
        DispatchQueue.main.asyncAfter(deadline: .now() + .milliseconds(2)) {
            MainActor.assumeIsolated { self.step() }
        }
    }

    private func finish() {
        // The last command buffers complete on Metal's threads a moment later.
        DispatchQueue.main.asyncAfter(deadline: .now() + .milliseconds(150)) {
            MainActor.assumeIsolated { self.finished(self.line()) }
        }
    }

    private func line() -> String {
        // The first frames warm the caches (shaped words, atlas, buffers); report the rest, as GPUI does.
        let steady = Array(samples.dropFirst(min(3, samples.count)))
        func ms(_ ns: UInt64) -> Double { Double(ns) / 1e6 }
        func mean(_ values: [UInt64]) -> Double { values.isEmpty ? 0 : values.reduce(0.0) { $0 + ms($1) } / Double(values.count) }
        func f(_ value: Double) -> String { String(format: "%.3f", value) }
        let build = steady.map(\.buildNs)
        let total = steady.map(\.totalNs)
        let gpu = view.renderer.gpuTimes.take()
        let gpuAverage = gpu.isEmpty ? 0 : gpu.reduce(0, +) / Double(gpu.count) * 1000
        let delta = view.renderer.counters - countersAtStart
        let rows = steady.isEmpty ? 0 : Double(steady.reduce(0) { $0 + $1.rowsRebuilt }) / Double(steady.count)
        let grid = view.renderer.gridSize
        return "frames n=\(samples.count) avg=\(f(mean(build)))ms max=\(f(ms(build.max() ?? 0)))ms "
            + "first=\(f(ms(samples.first?.buildNs ?? 0)))ms encode_avg=\(f(mean(steady.map(\.encodeNs))))ms "
            + "total_avg=\(f(mean(total)))ms total_max=\(f(ms(total.max() ?? 0)))ms cpu_avg=\(f(mean(steady.map(\.cpuNs))))ms "
            + "gpu_avg=\(f(gpuAverage))ms wait_avg=\(f(mean(steady.map(\.waitNs))))ms "
            + "fetch_avg=\(f(mean(steady.map(\.fetchNs))))ms apply_avg=\(f(mean(steady.map(\.applyNs))))ms "
            + "assemble_avg=\(f(mean(steady.map(\.assembleNs))))ms rows_rebuilt_avg=\(f(rows)) "
            + "words_shaped=\(delta.wordsShaped) glyphs_rasterised=\(delta.glyphsRasterised) atlas_resets=\(delta.atlasResets) "
            + "grid=\(grid.cols)x\(grid.rows)"
    }
}

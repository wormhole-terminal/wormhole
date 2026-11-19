// text-file: a benchmark readout for the developer (the control folder's `perf-profile`), not interface text
// What `perf-profile` records besides the draws, after the GPUI app's report (`perf/report.rs`): where the window's
// terminals run (two profiles compare only when these match), how many draws a second the run made, the Rust side's own
// share of a fetch (the screen copied under its lock, then packed), and what reading output cost over the run: the bytes
// the reader threads parsed and how fast, and how long the screens' lock was waited for and held, by the readers and by
// everyone else (`wormhole_term::perf`, every terminal of the process together).
//
// The reader's and the lock's counters exist only in a bridge built with its `perf-counters` feature; without it the report
// says they were not counted instead of printing zeros.

import Foundation
import TerminalMetal

#if WITH_UNIFFI
    import UniFFISource
    import WormholeBindings
#endif

@MainActor
struct PerfReading {
    /// The window's terminals in a session host and in the app's own process. A hosted terminal's program is read and parsed
    /// by its host, a process this profile does not see; the app parses the host's stream again, which is what is counted.
    let hosted: Int
    let local: Int
    private let startedNs = MonotonicClock.now()
    private let timingAtStart: (frames: UInt64, snapshotNs: UInt64, packNs: UInt64)?

    /// Starts counting. One profile at a time: the control folder runs its lines in order.
    init(view: MetalTerminalView, hosted: Int, local: Int) {
        (self.hosted, self.local) = (hosted, local)
        timingAtStart = Self.rustTiming(of: view)
        #if WITH_UNIFFI
            perfCountersStart()
        #endif
    }

    /// Stops counting: the lines for the table and the same numbers for the JSON, under GPUI's names.
    func stop(view: MetalTerminalView, draws: Int) -> (lines: String, json: [String: Any]) {
        let seconds = Double(MonotonicClock.now() - startedNs) / 1e9
        var lines = String(format: "  frames: per_second=%.1f", Self.per(Double(draws), seconds))
        var json: [String: Any] = ["terminals": ["hosted": hosted, "local": local], "frames_per_second": Self.per(Double(draws), seconds)]
        if let before = timingAtStart, let after = Self.rustTiming(of: view), after.frames > before.frames {
            let frames = Double(after.frames - before.frames)
            let snapshot = Double(after.snapshotNs - before.snapshotNs) / frames / 1e6, pack = Double(after.packNs - before.packNs) / frames / 1e6
            lines += String(format: "\n  fetch in Rust, per frame: snapshot=%.3fms pack=%.3fms (frames=%.0f)", snapshot, pack, frames)
            json["rust_fetch_ms"] = ["snapshot": snapshot, "pack": pack]
        }
        lines += "\n  terminals: hosted=\(hosted) local=\(local)"
        #if WITH_UNIFFI
            let counted = perfCountersStop()
            guard counted.available else {
                return (lines + "\n  reader: not counted (the bridge was built without its perf-counters feature)", json)
            }
            let megabytes = Double(counted.bytes) / 1e6, parsing = Double(counted.parseNs) / 1e9
            lines += String(
                format: "\n  reader: bytes=%llu mb_per_s=%.1f parse_mb_per_s=%.1f parse_busy_pct=%.1f slices=%llu", counted.bytes, Self.per(megabytes, seconds),
                Self.per(megabytes, parsing), 100 * Self.per(parsing, seconds), counted.slices)
            lines += "\n  reader lock: " + Self.line(counted.reader) + "\n  ui lock:     " + Self.line(counted.ui)
            json["reader"] = [
                "bytes": counted.bytes, "mb_per_s": Self.per(megabytes, seconds), "parse_mb_per_s": Self.per(megabytes, parsing),
                "parse_busy_pct": 100 * Self.per(parsing, seconds), "slices": counted.slices, "lock": Self.values(counted.reader),
            ]
            json["ui_lock"] = Self.values(counted.ui)
        #endif
        return (lines, json)
    }

    /// `a / b`, and 0 when there was no `b`.
    private static func per(_ a: Double, _ b: Double) -> Double { b > 0 ? a / b : 0 }

    private static func rustTiming(of view: MetalTerminalView) -> (frames: UInt64, snapshotNs: UInt64, packNs: UInt64)? {
        #if WITH_UNIFFI
            return (view.source as? UniFFIFrameSource)?.rustFrameTiming.map { ($0.frames, $0.snapshotNs, $0.packNs) }
        #else
            return nil
        #endif
    }

    #if WITH_UNIFFI
        private static func values(_ lock: PerfLockUse) -> [String: Any] {
            [
                "locks": lock.locks, "wait_ms": Double(lock.waitNs) / 1e6, "wait_max_ms": Double(lock.waitMaxNs) / 1e6,
                "hold_ms": Double(lock.holdNs) / 1e6, "hold_max_ms": Double(lock.holdMaxNs) / 1e6,
            ]
        }

        private static func line(_ lock: PerfLockUse) -> String {
            String(
                format: "locks=%llu wait=%.3fms (max %.3fms) hold=%.3fms (max %.3fms)", lock.locks, Double(lock.waitNs) / 1e6, Double(lock.waitMaxNs) / 1e6,
                Double(lock.holdNs) / 1e6, Double(lock.holdMaxNs) / 1e6)
        }
    #endif
}

// What the measuring mode records, and exactly what each number is. These are the definitions the README
// repeats; a number without them is not worth printing.
//
//   fetch   `FrameSource.frame()`: for the real source the Rust snapshot under the screen's lock plus the
//           pack and the lift into `Data`; for the mock a copy of its damaged rows.
//   build   parsing the header, copying the changed rows into the grid, and rebuilding their instances
//           (word cache, atlas misses included): everything between "bytes" and "instances".
//   encode  filling the GPU buffers from every row's instances, encoding the pass, and `commit`.
//   draw    fetch + build. This is the closest match to GPUI's `frames N`, which times `window.draw`: the
//           snapshot copy and the scene build, and not the platform's Metal encode that follows it.
//   total   fetch + build + encode.
//   cpu     the thread's CPU time (`CLOCK_THREAD_CPUTIME_ID`) over the same span as `total`; with no
//           waiting in the span it should equal `total` within the clock's noise.
//   gpu     the command buffer's `gpuEndTime - gpuStartTime`; GPUI does not report one.
//
// All durations are wall-clock nanoseconds from `CLOCK_UPTIME_RAW` unless named cpu.

import Darwin
import Foundation

public enum MonotonicClock {
    @inline(__always) public static func now() -> UInt64 { clock_gettime_nsec_np(CLOCK_UPTIME_RAW) }
    @inline(__always) public static func threadCPU() -> UInt64 { clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) }
}

public struct FrameSample: Sendable {
    public var fetchNs: UInt64
    public var buildNs: UInt64
    public var encodeNs: UInt64
    public var cpuNs: UInt64
    public var rowsRebuilt: Int
    public var changedRows: Int
    public var outcome: DrawOutcome

    public var drawNs: UInt64 { fetchNs + buildNs }
    public var totalNs: UInt64 { fetchNs + buildNs + encodeNs }
}

/// Counters the view keeps for the idle check and the report.
public struct ViewStatistics: Sendable, Equatable {
    /// Display link callbacks, and how many of them drew.
    public var ticks = 0
    public var framesDrawn = 0
    /// Main-queue hops made for the source's wake.
    public var wakes = 0
    /// Times the link paused because a tick found nothing new.
    public var pauses = 0
    public var busySkips = 0
    public var blinkFires = 0
}

public struct Summary: Sendable {
    public var count: Int
    public var mean: Double
    public var p50: Double
    public var p99: Double
    public var max: Double

    /// `values` in nanoseconds, summarised in milliseconds.
    public init(nanoseconds values: [UInt64]) {
        let sorted = values.sorted()
        count = sorted.count
        guard !sorted.isEmpty else {
            mean = 0; p50 = 0; p99 = 0; max = 0
            return
        }
        func ms(_ ns: UInt64) -> Double { Double(ns) / 1e6 }
        mean = values.reduce(0.0) { $0 + ms($1) } / Double(values.count)
        p50 = ms(sorted[sorted.count / 2])
        p99 = ms(sorted[min(sorted.count - 1, Int(Double(sorted.count) * 0.99))])
        max = ms(sorted[sorted.count - 1])
    }

    public var line: String {
        String(format: "avg=%.3fms p50=%.3fms p99=%.3fms max=%.3fms", mean, p50, p99, max)
    }
}

/// Process-wide wake-ups, from the kernel's own counters (what Activity Monitor's "Idle Wake Ups" column shows).
public enum WakeupCounter {
    public static func current() -> (interrupts: UInt64, platformIdle: UInt64)? {
        var info = task_power_info_v2_data_t()
        var count = mach_msg_type_number_t(MemoryLayout<task_power_info_v2_data_t>.size / MemoryLayout<natural_t>.size)
        let result = withUnsafeMutablePointer(to: &info) {
            $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
                task_info(mach_task_self_, task_flavor_t(TASK_POWER_INFO_V2), $0, &count)
            }
        }
        guard result == KERN_SUCCESS else { return nil }
        return (info.cpu_energy.task_interrupt_wakeups, info.cpu_energy.task_platform_idle_wakeups)
    }

    /// CPU time of the whole process so far, user plus system, in nanoseconds.
    public static func processCPUNanoseconds() -> UInt64 {
        var usage = rusage()
        getrusage(RUSAGE_SELF, &usage)
        func ns(_ t: timeval) -> UInt64 { UInt64(t.tv_sec) * 1_000_000_000 + UInt64(t.tv_usec) * 1000 }
        return ns(usage.ru_utime) + ns(usage.ru_stime)
    }
}

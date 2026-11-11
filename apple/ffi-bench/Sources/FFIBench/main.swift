// Measures what crossing the Swift/Rust seam costs, with no UI: the numbers behind spike step 2.
//
//   ffi-bench --ttystorm <path> [--cols 200] [--rows 69] [--hz 120] [--seconds 10]
//
// Needs WORMHOLE_DATA_DIR set to a folder named Wormhole-swift* (the library refuses otherwise).

import Darwin
import Foundation
import WormholeBindings

// MARK: clocks and statistics

func nowNs() -> UInt64 { clock_gettime_nsec_np(CLOCK_UPTIME_RAW) }

struct Stats {
    private(set) var samples: [UInt64] = []
    mutating func add(_ ns: UInt64) { samples.append(ns) }
    var count: Int { samples.count }
    func percentile(_ p: Double) -> UInt64 {
        guard !samples.isEmpty else { return 0 }
        let sorted = samples.sorted()
        return sorted[min(sorted.count - 1, Int(Double(sorted.count) * p))]
    }
    var mean: Double { samples.isEmpty ? 0 : Double(samples.reduce(0, +)) / Double(samples.count) }
    var max: UInt64 { samples.max() ?? 0 }
    func line(_ unit: Double = 1000, _ suffix: String = "us") -> String {
        func f(_ v: Double) -> String { String(format: "%.2f", v / unit) }
        return "n=\(count) mean=\(f(mean))\(suffix) p50=\(f(Double(percentile(0.5))))\(suffix) "
            + "p99=\(f(Double(percentile(0.99))))\(suffix) max=\(f(Double(max)))\(suffix)"
    }
}

// MARK: the wake listener

final class CountingListener: TerminalListener, @unchecked Sendable {
    private let lock = NSLock()
    private var wakes = 0
    private var lastWake: UInt64 = 0
    private let semaphore = DispatchSemaphore(value: 0)

    func wake() {
        lock.lock()
        wakes += 1
        lastWake = nowNs()
        lock.unlock()
        semaphore.signal()
    }
    var wakeCount: Int { lock.lock(); defer { lock.unlock() }; return wakes }
    var lastWakeTime: UInt64 { lock.lock(); defer { lock.unlock() }; return lastWake }
    func wait(timeout: Double) -> Bool { semaphore.wait(timeout: .now() + timeout) == .success }
    func drainSignals() { while semaphore.wait(timeout: .now()) == .success {} }
}

// MARK: arguments

var args = Array(CommandLine.arguments.dropFirst())
func option(_ name: String, default value: String) -> String {
    guard let i = args.firstIndex(of: name), i + 1 < args.count else { return value }
    return args[i + 1]
}
let cols = UInt16(option("--cols", default: "200"))!
let rows = UInt16(option("--rows", default: "69"))!
let hz = Double(option("--hz", default: "120"))!
let seconds = Double(option("--seconds", default: "10"))!
let ttystorm = option("--ttystorm", default: "ttystorm")

setbuf(stdout, nil)
print("frame format v\(frameFormatVersion()), grid \(cols)x\(rows), \(hz) Hz, \(seconds) s")

// MARK: 1. the cost of a call that does nothing

do {
    let n = 2_000_000
    var sink: UInt64 = 0
    let start = nowNs()
    for i in 0..<n { sink &+= ping(value: UInt64(i)) }
    let per = Double(nowNs() - start) / Double(n)
    print(String(format: "ping (u64 -> u64): %.1f ns per call (%d calls, checksum %llu)", per, n, sink))
}

// MARK: the floor: what allocating and copying a full frame costs in Swift with no FFI at all

do {
    let source = [UInt8](repeating: 7, count: 221_400)   // a full 200x69 frame
    var floor = Stats()
    var sink = 0
    for _ in 0..<5000 {
        let t0 = nowNs()
        let copy = source.withUnsafeBytes { Data($0) }
        floor.add(nowNs() - t0)
        sink &+= copy.count
    }
    print("floor: Data(copy of 221400 bytes), no FFI: \(floor.line()) (checksum \(sink))")
}

// MARK: helpers

func start(_ program: String, _ arguments: [String], listener: CountingListener) -> Terminal {
    do {
        return try Terminal.start(
            program: program, args: arguments, cols: cols, rows: rows,
            cellWidth: 8.4, cellHeight: 18.2, listener: listener)
    } catch {
        print("could not start \(program): \(error)")
        exit(1)
    }
}

/// Call `frame()` at `hz` for `seconds`, as a display link would; returns the per-call stats.
func frameLoop(_ terminal: Terminal, label: String) {
    var total = Stats(), bytes: [Int] = [], rustSnapshot = Stats(), rustPack = Stats()
    let period = UInt64(1e9 / hz)
    var next = nowNs()
    let end = next + UInt64(seconds * 1e9)
    var before = terminal.frameTiming()
    var changedRows = 0
    while next < end {
        // Sleep to the deadline (a sleeping thread, not a timer callback: the measurement is the call).
        let now = nowNs()
        if next > now { var ts = timespec(tv_sec: 0, tv_nsec: Int(next - now)); nanosleep(&ts, nil) }
        let t0 = nowNs()
        let data = terminal.frame()
        let t1 = nowNs()
        total.add(t1 - t0)
        bytes.append(data.count)
        let after = terminal.frameTiming()
        rustSnapshot.add(after.lastSnapshotNs)
        rustPack.add(after.lastPackNs)
        before = after
        // The changed-row count is the header's fifth word.
        data.withUnsafeBytes { raw in
            if raw.count >= 20 { changedRows += Int(raw.loadUnaligned(fromByteOffset: 16, as: UInt32.self)) }
        }
        next += period
        if next < t1 { next = t1 }
    }
    let meanBytes = Double(bytes.reduce(0, +)) / Double(max(bytes.count, 1))
    print("\(label):")
    print("  frame() total (Swift view of the whole call): \(total.line())")
    print("  of which in Rust: engine snapshot under lock:  \(rustSnapshot.line())")
    print("                    packing into bytes:          \(rustPack.line())")
    print(String(format: "  bytes per frame: mean %.0f (max %d); rows changed per frame: mean %.1f of %d",
                 meanBytes, bytes.max() ?? 0, Double(changedRows) / Double(max(bytes.count, 1)), rows))
    _ = before
}

// MARK: 2. frame() under a truecolor animation at the display's rate

let animation = option("--animation", default: "warptunnel")
do {
    let listener = CountingListener()
    let terminal = start(ttystorm, [animation, "--fps", "0", "--no-hud"], listener: listener)
    Thread.sleep(forTimeInterval: 2)   // settle
    _ = terminal.frame()
    frameLoop(terminal, label: "frame() under `ttystorm \(animation) --fps 0` at \(hz) Hz")

    // A full 200x69 copy, whatever changed: what a new surface or a resize costs.
    var full = Stats()
    var fullBytes = 0
    for _ in 0..<2000 {
        let t0 = nowNs()
        let data = terminal.fullFrame()
        full.add(nowNs() - t0)
        fullBytes = data.count
    }
    print("full_frame() (every row) under the same load: \(full.line()), \(fullBytes) bytes")
    terminal.close()
}

// MARK: 3. an idle terminal: frame() with nothing new, and drain_events()

do {
    let listener = CountingListener()
    let terminal = start("/bin/cat", [], listener: listener)
    Thread.sleep(forTimeInterval: 0.5)
    _ = terminal.frame()
    _ = terminal.drainEvents()
    var idle = Stats(), drain = Stats()
    var idleBytes = 0
    for _ in 0..<20000 {
        let t0 = nowNs()
        let data = terminal.frame()
        idle.add(nowNs() - t0)
        idleBytes = data.count
        let t2 = nowNs()
        _ = terminal.drainEvents()
        drain.add(nowNs() - t2)
    }
    print("idle frame() (nothing changed): \(idle.line()), \(idleBytes) bytes")
    print("drain_events() with nothing queued: \(drain.line())")

    // MARK: 4. wake latency: write a byte, `cat` echoes it, the reader parses it, the event thread wakes us

    var latency = Stats()
    var frameAfterWake = Stats()
    for i in 0..<2000 {
        listener.drainSignals()
        _ = terminal.drainEvents()
        let t0 = nowNs()
        terminal.write(bytes: Data("x".utf8))
        guard listener.wait(timeout: 2) else { print("no wake after write \(i)"); break }
        latency.add(listener.lastWakeTime - t0)
        let t1 = nowNs()
        _ = terminal.drainEvents()
        let data = terminal.frame()
        frameAfterWake.add(nowNs() - t1)
        _ = data
        if i % 60 == 59 { terminal.write(bytes: Data("\r".utf8)) }
    }
    print("wake latency (write -> pty -> cat echo -> reader -> parse -> event thread -> wake): \(latency.line())")
    print("drain_events() + frame() after a wake (one echoed byte): \(frameAfterWake.line())")
    terminal.close()
}

// MARK: 5. wakes under a flood: how many calls into Swift for how much output

do {
    let listener = CountingListener()
    let terminal = start(ttystorm, ["flood", "--kind", "logflood"], listener: listener)
    Thread.sleep(forTimeInterval: 1)
    let wakes0 = listener.wakeCount
    let period = UInt64(1e9 / hz)
    var next = nowNs()
    let end = next + UInt64(5 * 1e9)
    var drains = 0
    while next < end {
        let now = nowNs()
        if next > now { var ts = timespec(tv_sec: 0, tv_nsec: Int(next - now)); nanosleep(&ts, nil) }
        _ = terminal.drainEvents()
        _ = terminal.frame()
        drains += 1
        next += period
    }
    let wakes = listener.wakeCount - wakes0
    print("logflood for 5 s, drained at \(hz) Hz: \(drains) drains, \(wakes) wake() calls "
          + "(one per drain at most; the reader produced thousands of events)")
    terminal.close()
}
print("done")

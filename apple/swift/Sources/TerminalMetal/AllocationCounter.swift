// Counts the main thread's heap allocations while on, for the frame's allocation guard (a test) and `perf-profile`,
// as GPUI's counting allocator does for its frames. It hooks `malloc_logger`, the callback malloc itself calls on
// every allocation for its stack logging (what `malloc_history` reads): no allocator is replaced, and off it costs
// nothing. Other threads' allocations (the engine's reader) are not a frame's and are not counted.

import Darwin
import Synchronization

public enum AllocationCounter {
    private typealias Logger = @convention(c) (UInt32, UInt, UInt, UInt, UInt, UInt32) -> Void
    /// `MALLOC_LOG_TYPE_ALLOCATE` in libmalloc's stack logging.
    private static let allocate: UInt32 = 2
    private static let count = Atomic<Int>(0)
    private static let logger: Logger = { type, _, _, _, _, _ in
        if type & allocate != 0, pthread_main_np() == 1 { count.add(1, ordering: .relaxed) }
    }

    /// Allocations on the main thread since `start()`.
    public static var allocations: Int { count.load(ordering: .relaxed) }

    /// What the hook held before `start()` (stack logging of a profiled run), put back by `stop()`.
    private static let previous = Mutex<Logger?>(nil)

    private static func isOurs(_ other: Logger?) -> Bool {
        other.map { unsafeBitCast($0, to: UnsafeRawPointer.self) == unsafeBitCast(logger, to: UnsafeRawPointer.self) } ?? false
    }

    /// Starts counting from zero; false when malloc has no logger hook to set.
    @discardableResult
    public static func start() -> Bool {
        guard let slot else { return false }
        // Touched here, so the callback never runs a lazy initialiser (which could allocate) from inside malloc.
        count.store(0, ordering: .relaxed)
        _ = allocate
        if !isOurs(slot.pointee) { previous.withLock { $0 = slot.pointee } }
        slot.pointee = logger
        return true
    }

    /// Stops counting and gives the hook back to what held it before (Instruments, `MallocStackLogging`).
    public static func stop() {
        guard let slot, isOurs(slot.pointee) else { return }
        slot.pointee = previous.withLock { $0 }
    }

    /// Counts the allocations `body` makes on the main thread.
    public static func counting<T>(_ body: () throws -> T) rethrows -> (T, allocations: Int) {
        start()
        defer { stop() }
        let result = try body()
        return (result, allocations)
    }

    private static var slot: UnsafeMutablePointer<Logger?>? {
        dlsym(UnsafeMutableRawPointer(bitPattern: -2), "malloc_logger")?.assumingMemoryBound(to: Logger?.self)
    }
}

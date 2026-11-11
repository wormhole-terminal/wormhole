// Rust's calls block on the disk (a folder listing, a file read, a rename, the owner's dialog): they run on a
// background queue, never on the main actor and not on the cooperative pool, whose threads must stay free.

import Foundation

func offMain<T: Sendable>(_ work: @escaping @Sendable () -> T) async -> T {
    await withCheckedContinuation { continuation in
        DispatchQueue.global(qos: .userInitiated).async { continuation.resume(returning: work()) }
    }
}

// The FFI's git calls block on a process (each with core's timeout), so they run on a queue of their own: a
// blocked call must not hold one of the cooperative pool's few threads while a diff is waited for.

import Dispatch
import PanelModel
import WordsModel
import WormholeBindings

enum GitQueue {
    private static let queue = DispatchQueue(label: "wormhole.git", qos: .userInitiated, attributes: .concurrent)

    /// Runs `work` off the caller's executor and gives back its value; a failure of the FFI becomes the panel's
    /// `GitFailure` in git's words.
    static func run<T: Sendable>(_ work: @escaping @Sendable () throws -> T) async throws -> T {
        try await withCheckedThrowingContinuation { continuation in
            queue.async {
                do { continuation.resume(returning: try work()) } catch { continuation.resume(throwing: failure(error)) }
            }
        }
    }

    static func run<T: Sendable>(_ work: @escaping @Sendable () -> T) async -> T {
        await withCheckedContinuation { continuation in queue.async { continuation.resume(returning: work()) } }
    }

    static func failure(_ error: Error) -> GitFailure {
        switch error as? GitBackendError {
        case .NotARepository?: GitFailure(Words.text("git-not-a-repository", fallback: "Not a git repository"))
        case let .Failed(message)?: GitFailure(message)
        case let .NotIsolated(reason)?: GitFailure(reason)
        case nil: GitFailure("\(error)")
        }
    }
}

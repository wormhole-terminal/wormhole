// "Something that matters to `git status` changed" as an `AsyncStream`: core's rules over the OS's file events,
// debounced in Rust, one element per burst. A folder that is no repository yet (a `git init` still to come) is
// looked at with a stat walk, which costs no process, until it is one; then the watch starts.

import Foundation
import WormholeBindings

/// Rust calls `changed()` from the watch's thread: it only yields, and a burst that arrives while the reader is
/// busy stays one element.
private final class Relay: GitListener, @unchecked Sendable {
    let continuation: AsyncStream<Void>.Continuation
    init(_ continuation: AsyncStream<Void>.Continuation) { self.continuation = continuation }
    func changed() { continuation.yield() }
}

enum RepoChanges {
    /// How often a folder that is in no repository is looked at again.
    static let absentLook = Duration.seconds(3)

    static func stream(git: Git, folder: String) -> AsyncStream<Void> {
        AsyncStream(bufferingPolicy: .bufferingNewest(1)) { continuation in
            let holder = Holder()
            let task = Task {
                while !Task.isCancelled {
                    if git.isRepository(folder: folder), let watch = try? GitWatch.start(folder: folder, listener: Relay(continuation)) {
                        holder.set(watch)
                        // The repository appeared after the tab looked: read it now.
                        continuation.yield()
                        return
                    }
                    try? await Task.sleep(for: absentLook)
                }
            }
            continuation.onTermination = { _ in
                task.cancel()
                holder.stop()
            }
        }
    }
}

/// The watch once it started, so ending the stream ends it.
private final class Holder: @unchecked Sendable {
    private let lock = NSLock()
    private var watch: GitWatch?
    private var stopped = false

    func set(_ new: GitWatch) {
        lock.lock()
        defer { lock.unlock() }
        if stopped { new.stop() } else { watch = new }
    }

    func stop() {
        lock.lock()
        defer { lock.unlock() }
        stopped = true
        watch?.stop()
        watch = nil
    }
}

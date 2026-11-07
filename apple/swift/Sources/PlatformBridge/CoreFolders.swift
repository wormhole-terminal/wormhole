// The folder picker's providers and decorators from core (`folder_picker`): the typed-path walk, the multi-level
// fuzzy search, recent folders by frecency, chevrons and git branch badges. Each query is a job on a Rust thread
// under core's gate; cancelling the Swift task cancels the job, and the answer comes back through the task.

import Foundation
import PlatformModel
import Synchronization
import WormholeBindings

/// The picker's sources for one app: one provider per core provider, one decorator per core decorator, and the
/// recent folders they share.
public final class CoreFolders: Sendable {
    private let picker: FolderPicker

    init(recentsJSON: String?, recentTitle: String) {
        picker = FolderPicker(recentsJson: recentsJSON, recentTitle: recentTitle)
    }

    public var providers: [any FolderProviding] {
        (0..<picker.providerCount()).map { CoreProvider(picker: picker, index: $0) }
    }

    public var decorators: [any FolderDecorating] {
        (0..<picker.decoratorCount()).map { CoreDecorator(picker: picker, index: $0, kinds: picker.decoratorKinds(index: $0)) }
    }

    /// A terminal was in `path`: a visit, unless it is a place never recorded or a repeat within seconds.
    @discardableResult
    public func recordVisit(_ path: String, now: Date = Date()) -> Bool {
        picker.recordVisit(path: path, nowSecs: UInt64(now.timeIntervalSince1970))
    }

    /// Forget a recent folder (⌘⌫ on its row).
    @discardableResult
    public func forget(_ path: String) -> Bool { picker.forget(path: path) }

    /// The recent folders as the host keeps them; `PlatformBridge.makeFolders(recentsJSON:)` reads them back.
    public func recentsJSON() -> String { picker.recentsJson() }
}

/// A running job, cancellable from the task that asked: the job is handed over after it started, so a cancel that
/// came first is remembered and applied then.
private final class CancelBox: Sendable {
    private let state = Mutex<(cancelled: Bool, job: PickerJob?)>((false, nil))

    func set(_ job: PickerJob) {
        let cancelled = state.withLock { state -> Bool in
            state.job = job
            return state.cancelled
        }
        if cancelled { job.cancel() }
    }

    func cancel() {
        let job = state.withLock { state -> PickerJob? in
            state.cancelled = true
            return state.job
        }
        job?.cancel()
    }
}

/// Resumes its continuation with the one answer a job gives.
private final class Listener: PickerListener, Sendable {
    private let onAnswer: @Sendable (PickerAnswerRecord) -> Void
    private let onDecorations: @Sendable ([PickerDecorationRecord]) -> Void

    init(
        onAnswer: @escaping @Sendable (PickerAnswerRecord) -> Void = { _ in },
        onDecorations: @escaping @Sendable ([PickerDecorationRecord]) -> Void = { _ in }
    ) {
        (self.onAnswer, self.onDecorations) = (onAnswer, onDecorations)
    }

    func answered(answer: PickerAnswerRecord) { onAnswer(answer) }
    func decorated(found: [PickerDecorationRecord]) { onDecorations(found) }
}

private struct CoreProvider: FolderProviding {
    let picker: FolderPicker
    let index: UInt32

    func answer(_ query: PickerQuery) async -> PickerAnswer {
        let box = CancelBox()
        let record = await withTaskCancellationHandler {
            await withCheckedContinuation { continuation in
                box.set(picker.ask(index: index, query: query.record, listener: Listener(onAnswer: { continuation.resume(returning: $0) })))
            }
        } onCancel: {
            box.cancel()
        }
        return PickerAnswer(record)
    }
}

private struct CoreDecorator: FolderDecorating {
    let picker: FolderPicker
    let index: UInt32
    let kinds: [String]

    func decorate(_ keys: [String]) async -> [PickerDecoration] {
        let box = CancelBox()
        let found = await withTaskCancellationHandler {
            await withCheckedContinuation { continuation in
                box.set(picker.decorate(index: index, keys: keys, listener: Listener(onDecorations: { continuation.resume(returning: $0) })))
            }
        } onCancel: {
            box.cancel()
        }
        return found.map { PickerDecoration(key: $0.key, badge: $0.badge, chevron: $0.chevron) }
    }
}

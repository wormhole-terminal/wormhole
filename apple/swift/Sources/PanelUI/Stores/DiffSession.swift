// The diff of the one file that is open, whichever list it was opened from (Changes against the working tree or a
// branch, History in a commit): it fetches the text off the main actor, parses it, colours it, and keeps the last
// request only, so walking the files with the arrow keys never shows an older file's diff late.

import Observation
import PanelModel

@MainActor @Observable
public final class DiffSession {
    public enum Phase: Equatable { case idle, loading, shown(DiffFile?), failed(String) }

    public private(set) var file: ChangedFile?
    public private(set) var phase = Phase.idle
    @ObservationIgnored private var task: Task<Void, Never>?
    @ObservationIgnored private let highlighter: any SyntaxHighlighting

    public init(highlighter: any SyntaxHighlighting) { self.highlighter = highlighter }

    public var parsed: DiffFile? { if case let .shown(diff) = phase { diff } else { nil } }

    public func open(_ file: ChangedFile, fetch: @escaping @Sendable () async throws -> String) {
        self.file = file
        phase = .loading
        task?.cancel()
        let highlighter = highlighter
        task = Task { [weak self] in
            do {
                let text = try await fetch()
                let diff = await Task.detached(priority: .userInitiated) { () -> DiffFile? in
                    guard var parsed = UnifiedDiffParser.parse(text).first else { return nil }
                    DiffHighlighting.apply(to: &parsed, with: highlighter)
                    return parsed
                }.value
                guard !Task.isCancelled else { return }
                self?.phase = .shown(diff)
            } catch {
                guard !Task.isCancelled else { return }
                self?.phase = .failed((error as? GitFailure)?.message ?? "The diff can't be read.")
            }
        }
    }

    public func close() {
        task?.cancel()
        file = nil
        phase = .idle
    }
}

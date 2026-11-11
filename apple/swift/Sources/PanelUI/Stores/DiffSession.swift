// The diff of the one file that is open, whichever list it was opened from (Changes against the working tree or a
// branch, History in a commit): it fetches the parsed diff off the main actor, colours it, and keeps the last
// request only, so walking the files with the arrow keys never shows an older file's diff late.

import Observation
import PanelModel

@MainActor @Observable
public final class DiffSession {
    public enum Phase: Equatable { case idle, loading, shown(DiffFile?), failed(String) }

    public private(set) var file: ChangedFile?
    public private(set) var phase = Phase.idle
    /// Old against new in two columns (when there is room) and long lines wrapped: shared by the header and the
    /// rows, which the window draws apart when the diff covers the terminal.
    public var split: Bool {
        get { preferences.diffSplit }
        set { preferences.diffSplit = newValue }
    }
    public var wrap: Bool {
        get { preferences.diffWrap }
        set { preferences.diffWrap = newValue }
    }
    public let preferences: PanelPreferences
    @ObservationIgnored private var task: Task<Void, Never>?
    @ObservationIgnored private var reload: (@Sendable (Bool) async throws -> DiffFile)?
    @ObservationIgnored private let highlighter: any SyntaxHighlighting

    public init(highlighter: any SyntaxHighlighting, preferences: PanelPreferences = PanelPreferences()) {
        self.highlighter = highlighter
        self.preferences = preferences
    }

    public var parsed: DiffFile? { if case let .shown(diff) = phase { diff } else { nil } }

    /// `fetch(loadAnyway)` reads the diff; a patch core would not load unasked is read again with true ("Load diff").
    public func open(_ file: ChangedFile, fetch: @escaping @Sendable (_ loadAnyway: Bool) async throws -> DiffFile) {
        self.file = file
        reload = fetch
        read(loadAnyway: false)
    }

    public func loadAnyway() { read(loadAnyway: true) }

    private func read(loadAnyway: Bool) {
        guard let fetch = reload else { return }
        phase = .loading
        task?.cancel()
        let highlighter = highlighter
        let tabWidth = preferences.tabWidth
        task = Task { [weak self] in
            do {
                let fetched = try await fetch(loadAnyway)
                let diff = await Task.detached(priority: .userInitiated) { () -> DiffFile in
                    var colored = fetched
                    DiffHighlighting.apply(to: &colored, with: highlighter)
                    colored.widest = DiffRows.widest(colored, tabWidth: tabWidth)
                    return colored
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
        reload = nil
        file = nil
        phase = .idle
    }
}

// The diff of the one file that is open, whichever list it was opened from (Changes against the working tree or a
// branch, History in a commit): it fetches the parsed diff off the main actor, colours it, and keeps the last
// request only, so walking the files with the arrow keys never shows an older file's diff late.

import Observation
import PanelModel

/// The rows of a diff in both layouts and the digits its gutter needs: cut once, off the main actor, when the diff is
/// read (`diff/rows.rs`), so drawing and every sideways scroll step only pick a list.
public struct DiffBuilt: Sendable {
    public let unified: [DiffRow]
    public let split: [DiffRow]
    public let digits: Int

    init(_ diff: DiffFile) {
        (unified, split, digits) = (DiffRows.unified(diff), DiffRows.split(diff), DiffRows.numberDigits(diff))
    }
}

@MainActor @Observable
public final class DiffSession {
    public enum Phase: Equatable { case idle, loading, shown(DiffFile?), failed(String) }

    public private(set) var file: ChangedFile?
    public private(set) var phase = Phase.idle
    /// The status no longer lists the file: the diff stays as it was read, with "No longer changed" on it.
    public private(set) var gone = false
    /// The last re-read failed, so what shows may be out of date (the header's "Out of date", its tooltip says why).
    public private(set) var refreshError: String?
    /// What the file looked like when this diff was read (its size and time), to tell an edit from nothing.
    public private(set) var stamp: String?
    @ObservationIgnored private var forced = false
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
        preferences.tabObservers.append { [weak self] in self?.retab() }
    }

    /// `tab_width` changed: the widest line is measured again, so the sideways extent follows the tabs the rows draw.
    private func retab() {
        guard case .shown(let shown) = phase, var diff = shown else { return }
        diff.widest = DiffRows.widest(diff, tabWidth: preferences.tabWidth)
        phase = .shown(diff)
    }

    public var parsed: DiffFile? { if case let .shown(diff) = phase { diff } else { nil } }
    /// The rows of `parsed`, made with it.
    public private(set) var built: DiffBuilt?

    /// `fetch(loadAnyway)` reads the diff; a patch core would not load unasked is read again with true ("Load diff").
    public func open(_ file: ChangedFile, stamp: String? = nil, fetch: @escaping @Sendable (_ loadAnyway: Bool) async throws -> DiffFile) {
        self.file = file
        self.stamp = stamp
        reload = fetch
        (gone, refreshError, forced) = (false, nil, false)
        read(loadAnyway: false)
    }

    public func loadAnyway() {
        forced = true
        read(loadAnyway: true)
    }

    /// The file changed under the open diff (or its status entry did): read it again in place, so the scroll stays and the old
    /// rows show until the new ones are there. A failure leaves the old ones under "Out of date". What was loaded anyway stays so.
    public func reread(_ file: ChangedFile, stamp: String?) {
        self.file = file
        self.stamp = stamp
        gone = false
        read(loadAnyway: forced, inPlace: true)
    }

    public func markGone() { gone = true }

    private func read(loadAnyway: Bool, inPlace: Bool = false) {
        guard let fetch = reload else { return }
        if !inPlace { (phase, refreshError) = (.loading, nil) }
        task?.cancel()
        let highlighter = highlighter
        let tabWidth = preferences.tabWidth
        task = Task { [weak self] in
            do {
                let fetched = try await fetch(loadAnyway)
                let (diff, rows) = await Task.detached(priority: .userInitiated) { () -> (DiffFile, DiffBuilt) in
                    var colored = fetched
                    DiffHighlighting.apply(to: &colored, with: highlighter)
                    colored.widest = DiffRows.widest(colored, tabWidth: tabWidth)
                    return (colored, DiffBuilt(colored))
                }.value
                guard !Task.isCancelled else { return }
                self?.built = rows
                self?.phase = .shown(diff)
                self?.refreshError = nil
            } catch {
                guard !Task.isCancelled else { return }
                let why = (error as? GitFailure)?.message ?? panelText("diff-unreadable", "The diff can't be read.")
                if inPlace, self?.parsed != nil { self?.refreshError = why } else { self?.phase = .failed(why) }
            }
        }
    }

    public func close() {
        task?.cancel()
        reload = nil
        file = nil
        (phase, gone, refreshError, stamp, built) = (.idle, false, nil, nil, nil)
    }
}

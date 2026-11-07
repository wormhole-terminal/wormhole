// Find's state in one terminal (`terminal_view/find.rs`): the query, what it found, which match is current.
// The search runs behind `TerminalSearching`; a result from an older search is dropped (`generation`), and
// while the program keeps printing it runs again at most every `researchEvery`.

import Observation

@MainActor @Observable
public final class FindModel {
    public private(set) var isOpen = false
    public private(set) var text = ""
    public private(set) var options = FindOptions()
    public private(set) var outcome = FindOutcome()
    public private(set) var current: Int?
    public private(set) var isSearching = false
    /// Bumped to give the field the keyboard with its text selected (⌘F while open, ⌘G with nothing typed).
    public private(set) var focusRequest = 0

    @ObservationIgnored private let searcher: TerminalSearching
    @ObservationIgnored private var generation = 0
    @ObservationIgnored private var task: Task<Void, Never>?

    /// How often a terminal that keeps printing is searched again while the bar is open.
    static let researchEvery: Duration = .milliseconds(200)

    public init(searcher: TerminalSearching) { self.searcher = searcher }

    public var ticks: [Double] { isOpen ? searcher.ticks() : [] }

    /// ⌘F: opens the bar; `seed` (the selection) replaces the query.
    public func open(seed: String? = nil) {
        let reopened = !isOpen
        isOpen = true
        if reopened { searcher.setActive(true) }
        focusRequest += 1
        if let seed, seed != text {
            text = seed
            search(fresh: true)
        } else if reopened {
            search(fresh: true)
        }
    }

    /// ⌘E: the selection's first non-blank line becomes the query; the keyboard stays where it is.
    public func useSelectionForFind() {
        guard let line = searcher.selectionText()?.split(whereSeparator: \.isNewline)
            .first(where: { !$0.allSatisfy(\.isWhitespace) }) else { return }
        let seed = String(line)
        if !isOpen { searcher.setActive(true) }
        isOpen = true
        if seed != text { text = seed; search(fresh: true) }
    }

    public func close() {
        if isOpen { searcher.setActive(false) }
        isOpen = false
        task?.cancel()
        task = nil
        isSearching = false
        outcome = FindOutcome()
        current = nil
    }

    public func setText(_ new: String) {
        guard new != text else { return }
        text = new
        search(fresh: true)
    }

    public func toggle(_ option: FindOption) {
        options[option].toggle()
        search(fresh: true)
    }

    /// ⌘G (+1), ⇧⌘G (−1), ↩ and ⇧↩ in the field: round the ends. Closed or empty, the bar opens as ⌘F would.
    public func step(_ delta: Int) {
        guard isOpen, !text.isEmpty else { return open() }
        let count = outcome.count
        guard count > 0 else { return }
        let next: Int
        switch current {
        case let current?: next = ((current + delta) % count + count) % count
        case nil: next = delta < 0 ? count - 1 : 0
        }
        current = next
        searcher.reveal(match: next)
    }

    /// The program printed: search again soon, once per `researchEvery` however much it prints.
    public func outputChanged() {
        guard isOpen, !text.isEmpty, !outcome.invalid, task == nil, searcher.hasNewOutput else { return }
        task = Task { [weak self] in
            try? await Task.sleep(for: Self.researchEvery)
            guard !Task.isCancelled else { return }
            self?.task = nil
            self?.search(fresh: false)
        }
    }

    private func search(fresh: Bool) {
        generation += 1
        let mine = generation
        task?.cancel()
        isSearching = true
        let query = FindQuery(text: text, options: options)
        task = Task { [weak self, searcher] in
            let found = await searcher.search(query)
            guard let self, !Task.isCancelled, self.generation == mine, self.isOpen else { return }
            self.apply(found, fresh: fresh)
        }
    }

    private func apply(_ found: FindOutcome, fresh: Bool) {
        outcome = found
        isSearching = false
        task = nil
        if fresh {
            current = found.nearest
            if let nearest = found.nearest { searcher.reveal(match: nearest) }
        } else {
            current = found.count > 0 ? current.map { min($0, found.count - 1) } : nil
        }
        if searcher.hasNewOutput { outputChanged() }
    }
}

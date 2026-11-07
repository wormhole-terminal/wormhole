// What the Import page opens one terminal into: the settings it would bring over as rows a person ticks, what does not
// carry over and why, and the draft that keeps the ticks (`import/mod.rs`). Core reads the files and words the rows;
// nothing is written until "Import selected".

import Observation

public struct ImportReview: Sendable, Equatable {
    public struct Item: Sendable, Equatable {
        public var label: String
        public var from: String
        public var value: String
        /// A shell profile: the whole command, which runs in every new terminal.
        public var command: String?
        public var defaultOn: Bool

        public init(label: String, from: String, value: String, command: String? = nil, defaultOn: Bool = true) {
            (self.label, self.from, self.value, self.command, self.defaultOn) = (label, from, value, command, defaultOn)
        }
    }

    public struct Skip: Sendable, Equatable {
        public var key: String
        public var reason: String

        public init(key: String, reason: String) { (self.key, self.reason) = (key, reason) }
    }

    public var files: [String]
    public var items: [Item]
    public var skipped: [Skip]

    public init(files: [String] = [], items: [Item] = [], skipped: [Skip] = []) {
        (self.files, self.items, self.skipped) = (files, items, skipped)
    }
}

/// The page's state: which terminal is open, its review and ticks, and what the last import did.
@MainActor @Observable
public final class ImportDraft {
    public private(set) var open: String?
    public private(set) var reviews: [String: ImportReview] = [:]
    public private(set) var ticks: [String: [Bool]] = [:]
    /// The line the last import said, for the terminal it came from; or why it failed.
    public private(set) var outcome: (id: String, text: String, failed: Bool)?
    @ObservationIgnored private let host: any SettingsHost

    public init(host: any SettingsHost) { self.host = host }

    /// A scan again: what was read is read again when its card opens.
    public func reset() {
        (open, reviews, ticks, outcome) = (nil, [:], [:], nil)
    }

    /// Opens a card (reading its settings the first time), or closes it.
    public func toggle(_ id: String) {
        if open == id { open = nil; return }
        open = id
        guard reviews[id] == nil else { return }
        Task {
            let review = await host.importReview(id)
            reviews[id] = review ?? ImportReview()
            ticks[id] = (review?.items ?? []).map(\.defaultOn)
        }
    }

    public func isTicked(_ id: String, _ index: Int) -> Bool { ticks[id]?[index] ?? false }

    public func tick(_ id: String, _ index: Int, _ on: Bool) {
        ticks[id]?[index] = on
        outcome = nil
    }

    public func selected(_ id: String) -> Int { ticks[id]?.filter { $0 }.count ?? 0 }

    public func apply(_ id: String) {
        guard let review = reviews[id] else { return }
        let ticked = (ticks[id] ?? []).enumerated().filter(\.element).map(\.offset)
        let result = host.applyImport(id, ticked: ticked, shown: review.items.count)
        outcome = (id, result.text, result.failed)
    }
}

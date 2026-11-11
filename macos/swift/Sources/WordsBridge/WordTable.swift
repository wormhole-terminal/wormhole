// One language's words from `wormhole-text`: every message id and its text, fetched once (`textIds`,
// `textTable`) and then read by index or by id, with no call across the FFI. A message with variables is not in
// the table (Rust formats it by its typed function); `text(for:)` has nothing for it and the caller's fallback
// shows.

import Foundation
import WordsModel
import WormholeBindings

public final class WordTable: WordSource, Sendable {
    public let tag: String
    /// Every message id, in the order of the table (and of Rust's `Key` indices).
    public let ids: [String]
    public let texts: [String]
    /// A hash of the id list: a generated Swift key list made from another one is stale.
    public let hash: UInt64
    private let positions: [String: Int]

    /// The table of `tag`; nil for a language this build does not have.
    public init?(tag: String) {
        guard let texts = textTable(tag: tag) else { return nil }
        let ids = textIds()
        guard ids.count == texts.count else { return nil }
        (self.tag, self.ids, self.texts, hash) = (tag, ids, texts, textHash())
        positions = Dictionary(ids.enumerated().map { ($1, $0) }, uniquingKeysWith: { first, _ in first })
    }

    /// The table for the languages the person prefers (`Locale.preferredLanguages` as macOS lists them, most wanted
    /// first): the best shipped language, English when none.
    public static func preferring(_ requested: [String] = Locale.preferredLanguages) -> WordTable {
        // English is always there, so the second try cannot fail.
        WordTable(tag: textNegotiate(requested: requested)) ?? WordTable(tag: "en")!
    }

    public func text(for id: String) -> String? {
        positions[id].map { texts[$0] }.flatMap { $0.isEmpty ? nil : $0 }
    }

    public func format(_ id: String, _ args: [String: WordValue]) -> String? {
        textFormat(id: id, args: args.map { name, value in
            switch value {
            case .text(let text): TextArg(name: name, text: text, whole: nil)
            case .whole(let number): TextArg(name: name, text: "", whole: Int64(number))
            }
        })
    }

    /// The text at a `Key`'s index.
    public subscript(index: Int) -> String { texts[index] }
}

public enum WordsBridge {
    /// Makes the words of `tag` (or of the person's languages) the interface's, and makes Rust's own words
    /// (a banner's title) follow. Returns the table, for a language switch to compare revisions with.
    @discardableResult
    public static func install(tag: String? = nil) -> WordTable {
        let table = tag.flatMap(WordTable.init(tag:)) ?? WordTable.preferring()
        _ = textSetLanguage(tag: table.tag)
        Words.install(table)
        return table
    }
}

// Setting-level search, as `settings_index/search.rs` does it: every word of the query has to be found in a row's
// label, description, search words, config keys, or the titles of its group and section; where decides the order
// (a label that starts a word first, search words last). Outside the label a word has to start a word there
// ("line" finds "Lines kept", not "readline"). The FFI's `settings_search` replaces this behind the protocol.

import Foundation

public struct SearchHit: Sendable, Equatable, Identifiable {
    public let section: String
    /// The title of the group the row sits in, so rows alike by name (three "Font"s) can be told apart.
    public let group: String
    public let entry: SettingsSchema.Entry
    public let score: Int
    /// `Character` offsets to mark: the first query word in the label, else in the description.
    public let labelRange: Range<Int>?
    public let descriptionRange: Range<Int>?
    public var id: String { entry.id }
}

public protocol SettingsSearching: Sendable {
    func search(_ query: String, values: [String: JSONValue]) -> [SearchHit]
}

public struct SchemaSearch: SettingsSearching {
    public let schema: SettingsSchema
    public init(schema: SettingsSchema = .bundled) { self.schema = schema }

    private enum Points {
        static let labelWordStart = 100, labelInside = 70, key = 60, descriptionWordStart = 40
        static let keywords = 20, groupTitle = 15, sectionTitle = 10, labelPrefixBonus = 50
    }

    public func search(_ query: String, values: [String: JSONValue]) -> [SearchHit] {
        let words = query.lowercased().split(whereSeparator: \.isWhitespace).map(String.init)
        guard !words.isEmpty else { return [] }
        var hits: [SearchHit] = []
        for section in schema.sections {
            var found: [SearchHit] = []
            for group in section.groups {
                for entry in group.entries where entry.isShown(values: values) {
                    if let hit = match(words, entry, group: group.title, section: section) { found.append(hit) }
                }
            }
            hits += found.sorted { $0.score > $1.score }
        }
        return hits
    }

    private func match(
        _ words: [String], _ entry: SettingsSchema.Entry, group: String, section: SettingsSchema.Section
    ) -> SearchHit? {
        let label = entry.label.lowercased(), description = entry.description.lowercased()
        let keywords = entry.keywords.lowercased()
        var score = 0
        var labelRange: Range<Int>?, descriptionRange: Range<Int>?
        for word in words {
            var best = 0
            if let range = Self.find(word, in: label, wordStartOnly: false) {
                let atStart = Self.isWordStart(label, range.lowerBound)
                best = atStart ? Points.labelWordStart : Points.labelInside
                labelRange = labelRange ?? range
            }
            if best < Points.key, entry.keys.contains(where: { Self.find(word, in: $0, wordStartOnly: true) != nil }) {
                best = Points.key
            }
            if best < Points.descriptionWordStart, let range = Self.find(word, in: description, wordStartOnly: true) {
                best = Points.descriptionWordStart
                if labelRange == nil { descriptionRange = descriptionRange ?? range }
            }
            if best < Points.keywords, Self.find(word, in: keywords, wordStartOnly: true) != nil { best = Points.keywords }
            if best < Points.groupTitle, Self.find(word, in: group.lowercased(), wordStartOnly: true) != nil {
                best = Points.groupTitle
            }
            if best < Points.sectionTitle, Self.find(word, in: section.title.lowercased(), wordStartOnly: true) != nil {
                best = Points.sectionTitle
            }
            guard best > 0 else { return nil }
            score += best
        }
        if label.hasPrefix(words.joined(separator: " ")) { score += Points.labelPrefixBonus }
        return SearchHit(
            section: section.id, group: group, entry: entry, score: score, labelRange: labelRange,
            descriptionRange: labelRange == nil ? descriptionRange : nil)
    }

    /// Where `word` first sits in `text` (as `Character` offsets), at the start of a word or anywhere.
    static func find(_ word: String, in text: String, wordStartOnly: Bool) -> Range<Int>? {
        let characters = Array(text), needle = Array(word)
        guard !needle.isEmpty, characters.count >= needle.count else { return nil }
        for start in 0...(characters.count - needle.count) where characters[start..<start + needle.count].elementsEqual(needle) {
            if !wordStartOnly || isWordStart(text, start) { return start..<start + needle.count }
        }
        return nil
    }

    static func isWordStart(_ text: String, _ offset: Int) -> Bool {
        guard offset > 0 else { return true }
        let previous = text[text.index(text.startIndex, offsetBy: offset - 1)]
        return !(previous.isLetter || previous.isNumber)
    }
}

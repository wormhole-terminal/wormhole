// Setting-level search through core (`settings_index::search_visible`): the ranking, the rows a flag or an off switch
// keeps out and the span to mark all come from Rust; this looks each row up by id in the schema the window holds.

import SettingsModel
import WormholeBindings

public struct RustSettingsSearch: SettingsSearching {
    let handle: SettingsHandle
    let schema: SettingsSchema

    public func search(_ query: String, values: [String: JSONValue]) -> [SearchHit] {
        handle.search(query: query).compactMap { hit in
            guard let found = schema.entry(hit.entry) else { return nil }
            return SearchHit(
                section: hit.section, group: hit.group, entry: found.entry, score: Int(hit.score),
                labelRange: hit.label.flatMap { Self.characters($0, in: found.entry.label) },
                descriptionRange: hit.description.flatMap { Self.characters($0, in: found.entry.description) })
        }
    }

    /// Core's spans are UTF-8 byte offsets; the model's are `Character` offsets. Nil for a span that does not fit
    /// the text (a language that changed under it).
    static func characters(_ span: SettingsTextSpan, in text: String) -> Range<Int>? {
        let bytes = text.utf8
        guard span.start <= span.end, Int(span.end) <= bytes.count,
            let start = bytes.index(bytes.startIndex, offsetBy: Int(span.start), limitedBy: bytes.endIndex),
            let end = bytes.index(bytes.startIndex, offsetBy: Int(span.end), limitedBy: bytes.endIndex),
            let from = start.samePosition(in: text), let to = end.samePosition(in: text)
        else { return nil }
        let lower = text.distance(from: text.startIndex, to: from)
        return lower..<(lower + text.distance(from: from, to: to))
    }
}

// Several providers' answers as one list (`view::picker::merge`): rows that answer into a section with the same
// title are ranked together by score (stably), a section with a new title follows the ones before it, and a row
// two providers list is listed once. A slow provider holds back only its own rows: the model merges what is in.

import Foundation

public enum PickerMerge {
    public static func merge(_ answers: [PickerAnswer]) -> PickerAnswer {
        var merged = PickerAnswer()
        merged.sections = []
        var seen = Set<String>()
        for answer in answers {
            var place: [Int: Int] = [:]
            for (index, title) in answer.sections.enumerated() {
                if let existing = merged.sections.firstIndex(of: title), !title.isEmpty || merged.sections.count == 1 {
                    place[index] = existing
                } else {
                    merged.sections.append(title)
                    place[index] = merged.sections.count - 1
                }
            }
            for var row in answer.rows where seen.insert(row.key).inserted {
                row.section = place[row.section] ?? 0
                merged.rows.append(row)
            }
            merged.notes += answer.notes
            // The first answer's level is the folder's own; later ones only fill what it left empty.
            merged.up = merged.up ?? answer.up
            merged.upAtEnd = merged.upAtEnd || answer.upAtEnd
            if merged.crumbs.isEmpty { merged.crumbs = answer.crumbs }
        }
        if merged.sections.isEmpty { merged.sections = [""] }
        let order = merged.rows.enumerated().sorted { a, b in
            a.element.section != b.element.section
                ? a.element.section < b.element.section
                : a.element.score != b.element.score ? a.element.score > b.element.score : a.offset < b.offset
        }
        merged.rows = order.map(\.element)
        return merged
    }
}

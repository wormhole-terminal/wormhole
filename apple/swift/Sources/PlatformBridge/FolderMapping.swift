// Core's picker records as `PlatformModel`'s values, and back. Core marks bytes of a label; the picker marks
// characters.

import PlatformModel
import WormholeBindings

extension PickerQuery {
    var record: PickerQueryRecord {
        PickerQueryRecord(text: text, context: PickerContextRecord(cwd: context.cwd, home: context.home, profile: context.profile))
    }
}

extension PickerAnswer {
    init(_ record: PickerAnswerRecord) {
        self.init()
        sections = record.sections
        rows = record.rows.map(PickerRow.init)
        notes = record.notes.map(PickerNote.init)
        up = record.up
        upAtEnd = record.upAtEnd
        crumbs = record.crumbs.map { PickerCrumb(label: $0.label, text: $0.text) }
    }
}

extension PickerRow {
    init(_ record: PickerRowRecord) {
        self.init(key: record.key, label: record.label)
        marks = record.marks.map { characters($0, in: record.label) }
        icon = record.icon
        badge = record.badge
        detail = record.detail
        chevron = record.chevron
        descend = record.descend
        insert = record.insert
        section = Int(record.section)
        forgettable = record.forgettable
        kind = record.kind
        score = Int(record.score)
    }
}

extension PickerNote {
    init(_ record: PickerNoteRecord) {
        self =
            switch record {
            case .empty: .empty
            case .failed: .failed
            case .limited(let count): .limited(Int(count))
            case .searched(let count): .searched(Int(count))
            case .reading: .reading
            case .custom(let text): .custom(text)
            }
    }
}

// The picker's state (`view::picker::Model`): the text, the merged answer, the selection and what a key does.
// It runs nothing: the driver asks the providers (`FolderProviding`) off the main thread and hands the answers
// back by generation, so text that has moved on drops a late answer.

import Foundation

/// One line of the list.
public enum PickerLine: Equatable, Sendable {
    case header(String)
    case row(PickerRow)
    case note(String)
}

/// What a key or a click asks of the driver.
public enum PickerOutcome: Equatable, Sendable {
    case nothing
    case close
    /// Hand this to whoever opened the picker (a path to put at the cursor).
    case pick(String)
    /// Put this in the field (going into a row, or up a level) and ask again.
    case setText(String)
    case forget(key: String)
}

public struct PickerModel: Sendable {
    public private(set) var text = ""
    public private(set) var answer = PickerAnswer()
    public private(set) var selectedKey: String?
    /// Counts up with every change of text; an answer for an older one is dropped.
    public private(set) var generation = 0
    public let context: PickerContext

    public init(context: PickerContext) { self.context = context }

    /// The text changed: the query to ask the providers.
    public mutating func type(_ new: String) -> (query: PickerQuery, generation: Int) {
        text = new
        generation += 1
        return (PickerQuery(text: new, context: context), generation)
    }

    /// The merged answer for `generation`; false when the text has moved on.
    @discardableResult
    public mutating func receive(_ merged: PickerAnswer, generation: Int) -> Bool {
        guard generation == self.generation else { return false }
        answer = merged
        if answer.rows.isEmpty { answer.notes.append(.empty) }
        if !answer.rows.contains(where: { $0.key == selectedKey }) { selectedKey = answer.rows.first { $0.insert != nil }?.key }
        return true
    }

    public mutating func decorate(_ found: [PickerDecoration]) {
        for decoration in found {
            guard let at = answer.rows.firstIndex(where: { $0.key == decoration.key }) else { continue }
            if let badge = decoration.badge { answer.rows[at].badge = badge }
            if let chevron = decoration.chevron { answer.rows[at].chevron = chevron }
        }
    }

    public var selected: PickerRow? { answer.rows.first { $0.key == selectedKey } }

    /// The lines of the list: each section under its title (none for an empty one), then the notes.
    public var lines: [PickerLine] {
        var lines: [PickerLine] = []
        for (index, title) in answer.sections.enumerated() {
            let rows = answer.rows.filter { $0.section == index }
            guard !rows.isEmpty else { continue }
            if !title.isEmpty { lines.append(.header(title)) }
            lines += rows.map(PickerLine.row)
        }
        return lines
    }

    public var facts: PickerFacts {
        var facts = PickerFacts()
        facts.up = answer.up != nil
        facts.upAtEnd = answer.upAtEnd
        facts.descends = selected?.descend != nil
        facts.ghost = ghost != nil
        facts.forgettable = selected?.forgettable ?? false
        return facts
    }

    /// The grey completion of the selected row after the caret.
    public var ghost: String? { selected?.descend.flatMap { PickerKeys.ghost(typed: text, completion: $0) } }

    /// Do what a key decided (`PickerKeys.decide`).
    public mutating func apply(_ action: PickerDo) -> PickerOutcome {
        switch action {
        case .pass: return .nothing
        case .close: return .close
        case .step(let delta): step(delta); return .nothing
        case .pick: return selected?.insert.map(PickerOutcome.pick) ?? .nothing
        case .descend: return selected?.descend.map(PickerOutcome.setText) ?? .nothing
        case .up: return answer.up.map(PickerOutcome.setText) ?? .nothing
        case .forget: return selected.map { .forget(key: $0.key) } ?? .nothing
        }
    }

    /// Move the selection over the rows that can be picked, stopping at the ends.
    public mutating func step(_ delta: Int) {
        let pickable = answer.rows.filter { $0.insert != nil }
        guard !pickable.isEmpty else { return }
        let current = pickable.firstIndex { $0.key == selectedKey } ?? (delta > 0 ? -1 : pickable.count)
        selectedKey = pickable[min(max(current + delta, 0), pickable.count - 1)].key
    }

    /// Select the row a click was on.
    public mutating func select(_ key: String) {
        if answer.rows.contains(where: { $0.key == key && $0.insert != nil }) { selectedKey = key }
    }
}

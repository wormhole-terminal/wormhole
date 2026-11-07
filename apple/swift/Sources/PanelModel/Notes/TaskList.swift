// A terminal's next steps: added to, ticked and deleted. Pure value logic; the view animates the changes.

import Foundation

public struct PanelTask: Identifiable, Hashable, Sendable {
    public let id: UUID
    public var text: String
    public var done: Bool

    public init(id: UUID = UUID(), text: String, done: Bool = false) {
        self.id = id
        self.text = text
        self.done = done
    }
}

public struct TaskList: Equatable, Sendable {
    public private(set) var items: [PanelTask]

    public init(_ items: [PanelTask] = []) { self.items = items }

    public var remaining: Int { items.filter { !$0.done }.count }

    /// Blank input adds nothing: pressing return on an empty field is not a request for an empty task.
    @discardableResult
    public mutating func add(_ text: String) -> PanelTask? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }
        let task = PanelTask(text: trimmed)
        items.append(task)
        return task
    }

    public mutating func toggle(_ id: PanelTask.ID) {
        guard let index = items.firstIndex(where: { $0.id == id }) else { return }
        items[index].done.toggle()
    }

    /// Editing a step to nothing removes it, as emptying a field does in the real panel.
    public mutating func rename(_ id: PanelTask.ID, to text: String) {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let index = items.firstIndex(where: { $0.id == id }) else { return }
        if trimmed.isEmpty { items.remove(at: index) } else { items[index].text = trimmed }
    }

    public mutating func delete(_ id: PanelTask.ID) {
        items.removeAll { $0.id == id }
    }

    /// A step goes where another was (drag and drop, ⌥⌘↑ and ⌥⌘↓).
    public mutating func move(from: Int, to: Int) { Reorder.move(&items, from: from, to: to) }

    /// Done steps leave in one go; open ones keep their order.
    public mutating func clearDone() {
        items.removeAll(where: \.done)
    }
}

/// One line of the Next steps card (`view::panel::steps`).
public enum StepRow: Identifiable, Equatable, Sendable {
    case step(PanelTask)
    /// "N done": the done steps folded under the open ones.
    case doneFold(Int)

    public var id: String {
        switch self {
        case let .step(task): task.id.uuidString
        case .doneFold: "done-fold"
        }
    }
}

extension TaskList {
    /// More done steps than this fold into one "N done" row after the open ones.
    public static let doneFoldAbove = 5

    /// Every step where it is while few are done; otherwise the open ones, then the "N done" row, with the done
    /// steps under it only while `showDone`.
    public func rows(showDone: Bool) -> [StepRow] {
        let done = items.filter(\.done)
        guard done.count > Self.doneFoldAbove else { return items.map(StepRow.step) }
        var rows = items.filter { !$0.done }.map(StepRow.step)
        rows.append(.doneFold(done.count))
        if showDone { rows += done.map(StepRow.step) }
        return rows
    }
}

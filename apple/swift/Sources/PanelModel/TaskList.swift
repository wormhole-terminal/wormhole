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

    public mutating func delete(_ id: PanelTask.ID) {
        items.removeAll { $0.id == id }
    }

    /// Done steps leave in one go; open ones keep their order.
    public mutating func clearDone() {
        items.removeAll(where: \.done)
    }
}

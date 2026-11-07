// What the Notes tab shows for one terminal: free text and the next steps.

public struct NotesModel: Equatable, Sendable {
    public var text: String
    public var tasks: TaskList

    public init(text: String = "", tasks: TaskList = TaskList()) {
        self.text = text
        self.tasks = tasks
    }
}

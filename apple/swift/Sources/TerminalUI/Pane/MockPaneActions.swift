// Records what a pane bar's buttons were asked, for tests and the gallery.

@MainActor
public final class MockPaneActions: PaneBarActions {
    public private(set) var calls: [String] = []
    public init() {}

    public func answerClipboard(_ choice: ClipboardChoice) { calls.append("clipboard \(choice)") }
    public func tryAttachAgain() { calls.append("retry") }
    public func openNewTerminalHere() { calls.append("new") }
    public func endHost(hard: Bool) { calls.append(hard ? "end-hard" : "end") }
    public func waitForHost() { calls.append("wait") }
    public func restart() { calls.append("restart") }
    public func closeTerminal() { calls.append("close") }
    public func dismissHostNotice() { calls.append("dismiss") }
}

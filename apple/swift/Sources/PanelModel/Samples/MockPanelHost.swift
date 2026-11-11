// A `PanelHost` that records what it was asked, for tests and previews.

@MainActor
public final class MockPanelHost: PanelHost {
    public private(set) var copied: [String] = []
    public private(set) var revealed: [String] = []
    public private(set) var opened: [String] = []
    public private(set) var sent: [String] = []
    public private(set) var pasted: [String] = []
    public private(set) var notices: [String] = []
    public private(set) var terminals: [String] = []
    public var prompt: String?
    public var acceptsPaste = true
    public var folder: String?
    public let editorName: String? = "Zed"
    public let homeFolder: String? = "/Users/wanderer"

    public init() {}

    public func copy(_ text: String) { copied.append(text) }
    public func reveal(_ path: String) { revealed.append(path) }
    public func open(_ path: String, line: Int?) { opened.append(line.map { "\(path):\($0)" } ?? path) }
    public func open(_ path: String, inBundle bundle: String) { opened.append("\(bundle) \(path)") }
    public private(set) var commandsShown = 0
    public func showProjectCommands() { commandsShown += 1 }
    public func paste(_ text: String) -> Bool {
        guard acceptsPaste else { return false }
        pasted.append(text)
        return true
    }
    public func send(_ text: String) { sent.append(text) }
    public func submit() { sent.append("\r") }
    public func takePrompt() -> String? { defer { prompt = nil }; return prompt }
    public func focusTerminal() {}
    public func chooseFolder() async -> String? { folder }
    public func notify(_ text: String) { notices.append(text) }
    public func openTerminal(in folder: String, running command: String?) { terminals.append(folder) }
}

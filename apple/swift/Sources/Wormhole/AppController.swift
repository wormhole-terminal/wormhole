// The app: opens the workspace window over the chosen sources and keeps the control folder and the idle check.

import AppKit
import SwiftUI
import TerminalCore
import TerminalMetal
import WindowUI
import WorkspaceModel

@MainActor
final class AppController: NSObject, NSApplicationDelegate {
    let options: Options
    var devctl: DevCtl?
    private(set) var store = LocalStore(terminals: [], collapsed: false)
    private(set) var terminals = TerminalViews { _ in nil }
    private var windowController: WorkspaceWindowController?
    private var configuration: RendererConfiguration?

    init(options: Options) {
        self.options = options
    }

    var window: NSWindow? { windowController?.window }
    /// The selected terminal's view, if it exists: what the control commands act on.
    var selectedView: MetalTerminalView? { store.state.selected.flatMap { terminals.existing($0) } }

    func applicationDidFinishLaunching(_ notification: Notification) {
        do {
            try open()
        } catch {
            FileHandle.standardError.write(Data("cannot start: \(error)\n".utf8))
            exit(1)
        }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
    func applicationWillTerminate(_ notification: Notification) { terminals.shutdown() }

    // MARK: opening

    private func open() throws {
        let (configuration, note) = options.rendererConfiguration()
        if let note { FileHandle.standardError.write(Data((note + "\n").utf8)) }
        self.configuration = configuration
        // A fixed grid is a measuring mode: with the columns closed, the terminal is all the window shows.
        store = LocalStore(terminals: Self.initialTerminals(real: options.program != nil), collapsed: options.grid != nil)
        terminals = TerminalViews { [weak self] id in self?.makeView(for: id) }
        guard let first = store.state.selected.flatMap({ terminals.view(for: $0) }) else {
            throw Options.UsageError(description: "no terminal source could start")
        }

        let content = StoreRoot(store: store, terminals: terminals)
        let terminalSize = options.grid.map { first.renderer.pointSize(forGrid: $0.cols, $0.rows) } ?? CGSize(width: 960, height: 600)
        // The title row and the status bar sit above and below the terminal: 38 + 24 pt.
        let size = CGSize(width: terminalSize.width, height: terminalSize.height + 62)
        let controller = WorkspaceWindowController(title: "Wormhole (Swift experiment)", contentSize: size, content: content)
        controller.onClose = { [weak self] in self?.terminals.shutdown() }
        windowController = controller
        if options.background {
            // A test instance: behind the user's windows, never the active app.
            controller.window?.orderBack(nil)
        } else {
            NSApp.mainMenu = AppMenu.make(target: self)
            controller.showWindow(nil)
            NSApp.activate()
        }

        if let directory = options.devctl {
            let devctl = DevCtl(directory: directory) { [weak self] line in self?.run(command: line) }
            try devctl.start()
            self.devctl = devctl
        }
        if let seconds = options.idleSeconds { scheduleIdleReport(after: seconds) }
    }

    /// Some rows to look at when the sources are mock; a real shell starts with the one.
    private static func initialTerminals(real: Bool) -> [TerminalSummary] {
        let first = TerminalSummary(id: 1, title: "zsh", subtitle: "~/Projects/wormhole", branch: "main")
        guard !real else { return [first] }
        return [
            first,
            TerminalSummary(id: 2, title: "agent", subtitle: "~/Projects/wormhole", branch: "exp/swift-ui", activity: .working),
            TerminalSummary(id: 3, title: "build", subtitle: "~/Projects/web", branch: "main", activity: .needsInput),
        ]
    }

    private func makeView(for id: TerminalID) -> MetalTerminalView? {
        guard let configuration else { return nil }
        do {
            // Each mock terminal draws from its own seed; the first keeps the one given.
            var options = options
            options.seed &+= UInt64(max(0, id - 1))
            let source = try SourceFactory.make(options, size: GridSize(cols: 100, rows: 30))
            let view = try MetalTerminalView(source: source, configuration: configuration)
            view.fixedGrid = options.grid
            view.onTitle = { [weak self] title in self?.store.setTitle(title, of: id) }
            view.onExit = { [weak self] code in
                self?.devctl?.append("exited terminal=\(id) code=\(code.map(String.init) ?? "unknown")")
            }
            return view
        } catch {
            FileHandle.standardError.write(Data("terminal \(id): cannot start: \(error)\n".utf8))
            return nil
        }
    }

    // MARK: menu actions

    @objc func toggleSidebar() { store.send(.toggleSidebar) }
    @objc func togglePanel() { store.send(.togglePanel) }
    @objc func newTerminal() { store.send(.newTerminal) }

    @objc func closeTerminal() {
        guard let id = store.state.selected else { return }
        store.send(.close(id))
        if store.state.terminals.isEmpty { window?.close() }
    }

    @objc func selectNumbered(_ sender: NSMenuItem) {
        guard store.state.terminals.indices.contains(sender.tag - 1) else { return }
        store.send(.select(store.state.terminals[sender.tag - 1].id))
    }
}

/// Reads the store inside a body, so a change to its state redraws the window.
private struct StoreRoot: View {
    let store: LocalStore
    let terminals: TerminalViews

    var body: some View {
        WorkspaceRootView(state: store.state, send: { store.send($0) }, terminals: terminals)
    }
}

// The window: one terminal view over the chosen source, the control folder, and the idle check.

import AppKit
import TerminalCore
import TerminalMetal

@MainActor
final class AppController: NSObject, NSApplicationDelegate, NSWindowDelegate {
    private let options: Options
    private var window: NSWindow?
    private var view: MetalTerminalView?
    private var devctl: DevCtl?

    init(options: Options) {
        self.options = options
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        do {
            try open()
        } catch {
            FileHandle.standardError.write(Data("cannot start: \(error)\n".utf8))
            exit(1)
        }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    func applicationWillTerminate(_ notification: Notification) {
        view?.shutdown()
    }

    func windowWillClose(_ notification: Notification) {
        view?.shutdown()
    }

    // MARK: opening

    private func open() throws {
        let (configuration, note) = options.rendererConfiguration()
        if let note { FileHandle.standardError.write(Data((note + "\n").utf8)) }
        let source = try SourceFactory.make(options, size: GridSize(cols: 100, rows: 30))
        let view = try MetalTerminalView(source: source, configuration: configuration)
        view.fixedGrid = options.grid
        view.onTitle = { [weak self] title in self?.window?.title = title.isEmpty ? "Wormhole (Swift experiment)" : title }
        view.onExit = { [weak self] code in
            self?.devctl?.append("exited code=\(code.map(String.init) ?? "unknown")")
        }
        self.view = view

        let content = options.grid.map { view.renderer.pointSize(forGrid: $0.cols, $0.rows) } ?? CGSize(width: 960, height: 600)
        let window = NSWindow(
            contentRect: NSRect(origin: .zero, size: content), styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered, defer: false)
        window.title = "Wormhole (Swift experiment)"
        window.contentView = view
        window.delegate = self
        window.isReleasedWhenClosed = false
        window.center()
        window.makeFirstResponder(view)
        self.window = window
        if options.background {
            // A test instance: behind the user's windows, never the active app.
            window.orderBack(nil)
        } else {
            installMenu()
            window.makeKeyAndOrderFront(nil)
            NSApp.activate()
        }

        if let directory = options.devctl {
            let devctl = DevCtl(directory: directory) { [weak self] line in self?.run(command: line) }
            try devctl.start()
            self.devctl = devctl
        }
        if let seconds = options.idleSeconds { scheduleIdleReport(after: seconds) }
    }

    private func installMenu() {
        let main = NSMenu()
        let app = NSMenuItem()
        main.addItem(app)
        let appMenu = NSMenu()
        appMenu.addItem(withTitle: "Quit", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        app.submenu = appMenu
        let edit = NSMenuItem()
        main.addItem(edit)
        let editMenu = NSMenu(title: "Edit")
        editMenu.addItem(withTitle: "Paste", action: #selector(MetalTerminalView.paste(_:)), keyEquivalent: "v")
        edit.submenu = editMenu
        NSApp.mainMenu = main
    }

    // MARK: control commands

    private func run(command line: String) {
        guard let view else { return }
        let parts = line.split(separator: " ", maxSplits: 1).map(String.init)
        let argument = parts.count > 1 ? parts[1] : ""
        switch parts.first ?? "" {
        case "frames":
            FramesRun(view: view, count: Int(argument) ?? 40) { [weak self] line in
                self?.devctl?.append(line, to: "frames.txt")
            }.start()
            // The run keeps itself alive through its scheduled steps.
        case "shot":
            guard !argument.isEmpty else { return reply("error: shot needs a path") }
            do {
                try view.writeScreenshot(to: URL(fileURLWithPath: argument))
                reply("shot \(argument)")
            } catch {
                reply("error: shot failed: \(error)")
            }
        case "state": reply(state(view))
        case "screen":
            let grid = view.renderer.gridSize
            reply((0..<grid.rows).map { view.renderer.text(ofRow: $0) }.joined(separator: "\n"))
        case "type":
            view.source.write(Data(argument.replacingOccurrences(of: "\\r", with: "\r").replacingOccurrences(of: "\\n", with: "\r").utf8))
            reply("typed")
        case "quit":
            reply("quit")
            NSApp.terminate(nil)
        default:
            reply("error: no command \(line)")
        }
    }

    private func reply(_ text: String) { devctl?.append(text) }

    private func state(_ view: MetalTerminalView) -> String {
        let renderer = view.renderer
        let grid = renderer.gridSize, cell = renderer.cellPixels, stats = view.statistics, counters = renderer.counters
        return "grid=\(grid.cols)x\(grid.rows) cell=\(cell.width)x\(cell.height)px scale=\(renderer.scale) "
            + "frames_drawn=\(stats.framesDrawn) ticks=\(stats.ticks) wakes=\(stats.wakes) pauses=\(stats.pauses) "
            + "busy_skips=\(stats.busySkips) blink_fires=\(stats.blinkFires) link=\(view.isLinkRunning ? "running" : "paused") "
            + "words_shaped=\(counters.wordsShaped) word_hits=\(counters.wordHits) glyphs_rasterised=\(counters.glyphsRasterised) "
            + "atlas_resets=\(counters.atlasResets)"
    }

    // MARK: the idle check

    /// Hold the window idle, then print what the kernel counted: process CPU time and wake-ups. A one-shot
    /// timer at each end of the interval is the only thing this adds to the process.
    private func scheduleIdleReport(after seconds: Double) {
        guard let view else { return }
        // Let the first frames and any start-up work finish, then stop the source: the window has nothing to show.
        DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
            MainActor.assumeIsolated {
                view.source.stop()
                let cpu0 = WakeupCounter.processCPUNanoseconds()
                let wake0 = WakeupCounter.current()
                let stats0 = view.statistics
                DispatchQueue.main.asyncAfter(deadline: .now() + seconds) {
                    MainActor.assumeIsolated {
                        let cpu = WakeupCounter.processCPUNanoseconds() - cpu0
                        let stats = view.statistics
                        var report = String(
                            format: "idle %.1fs: cpu=%.2fms ticks=%d frames=%d wakes=%d blink=%d link=%@",
                            seconds, Double(cpu) / 1e6, stats.ticks - stats0.ticks, stats.framesDrawn - stats0.framesDrawn,
                            stats.wakes - stats0.wakes, stats.blinkFires - stats0.blinkFires, view.isLinkRunning ? "running" : "paused")
                        if let wake0, let wake1 = WakeupCounter.current() {
                            report += " interrupt_wakeups=\(wake1.interrupts - wake0.interrupts)"
                                + " platform_idle_wakeups=\(wake1.platformIdle - wake0.platformIdle)"
                        }
                        print(report)
                        self.devctl?.append(report)
                        NSApp.terminate(nil)
                    }
                }
            }
        }
    }
}

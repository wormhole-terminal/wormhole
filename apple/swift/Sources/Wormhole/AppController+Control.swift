// The control folder's commands (`WORMHOLE_DEVCTL`): the renderer's as before, plus the window's.

import AppKit
import Foundation
import TerminalCore
import TerminalMetal
import WorkspaceModel

extension AppController {
    func run(command line: String) {
        let parts = line.split(separator: " ", maxSplits: 1).map(String.init)
        let command = parts.first ?? ""
        let argument = parts.count > 1 ? parts[1] : ""
        // A window behind others is not drawn; the commands that read it want what the last change made.
        window?.contentView?.layoutSubtreeIfNeeded()
        if runLook(command: command, argument: argument) || runWindow(command: command, argument: argument) || runWindows(command: command, argument: argument)
            || runWorkspace(command: command, argument: argument) || runSettings(command: command, argument: argument) || runPanel(command: command, argument: argument) || runCore(command: command, argument: argument)
            || runScripted(command: command, argument: argument) || runVerify(command: command, argument: argument)
            || runKeys(command: command, argument: argument)
        { return }
        guard let view = selectedView else { return reply("error: no terminal is shown") }
        run(command: command, argument: argument, on: view)
    }

    private func run(command: String, argument: String, on view: MetalTerminalView) {
        switch command {
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
        // A background instance is never the key window, where a program's clipboard read is refused: this says it is in front.
        case "clipboard-front": view.source.setOnScreen(argument == "on"); reply("clipboard-front \(argument)")
        case "context-menu":
            // What a right-click on the selected terminal would show: each item's title, key and whether it is on.
            guard let id = hub.state.selected, let menu = terminalMenu(for: id) else { return reply("error: no menu") }
            reply(menu.items.map { $0.isSeparatorItem ? "---" : "\($0.title) [\($0.keyEquivalent == "" ? "-" : $0.keyEquivalent)\($0.isEnabled ? "" : " off")]" }.joined(separator: "\n"))
        case "block": reply(hub.blockTargetSummary)
        case "clipboard-answer":
            guard let choice = ["once": ClipboardChoice.once, "always": .always, "deny": .deny][argument] else { return reply("error: once, always or deny") }
            hub.answerClipboard(choice)
            reply("clipboard-answer \(argument)")
        case "select-cells":
            let n = argument.split(separator: " ").compactMap { Int($0) }
            guard n.count == 4 else { return reply("error: select-cells ROW COL ROW COL") }
            view.select(from: (n[0], n[1]), to: (n[2], n[3]))
            reply("select-cells \(argument)")
        case "state": reply(state(view))
        case "screen":
            let grid = view.renderer.gridSize
            reply((0..<grid.rows).map { view.renderer.text(ofRow: $0) }.joined(separator: "\n"))
        case "type":
            view.source.write(Data(argument.replacingOccurrences(of: "\\r", with: "\r").replacingOccurrences(of: "\\n", with: "\r").utf8))
            reply("typed")
        default:
            reply("error: no command \(command) \(argument)")
        }
    }

    func reply(_ text: String) { devctl?.append(text) }

    private func state(_ view: MetalTerminalView) -> String {
        let renderer = view.renderer
        let grid = renderer.gridSize, cell = renderer.cellPixels, stats = view.statistics, counters = renderer.counters
        let shown = hub.state
        return "grid=\(grid.cols)x\(grid.rows) cell=\(cell.width)x\(cell.height)px scale=\(renderer.scale) "
            + "frames_drawn=\(stats.framesDrawn) ticks=\(stats.ticks) wakes=\(stats.wakes) pauses=\(stats.pauses) "
            + "busy_skips=\(stats.busySkips) blink_fires=\(stats.blinkFires) link=\(view.isLinkRunning ? "running" : "paused") "
            + "words_shaped=\(counters.wordsShaped) word_hits=\(counters.wordHits) glyphs_rasterised=\(counters.glyphsRasterised) "
            + "atlas_resets=\(counters.atlasResets) "
            + "terminals=\(shown.live.count) selected=\(shown.selected.map(String.init) ?? "none") "
            + "sidebar=\(shown.sidebarVisible) panel=\(shown.panelVisible) tab=\(shown.panelTab.rawValue) "
            + "window=\(window?.windowNumber ?? 0) " + lookState + " " + hub.paneState + " focus=\(hub.focus.detail)"
    }
}

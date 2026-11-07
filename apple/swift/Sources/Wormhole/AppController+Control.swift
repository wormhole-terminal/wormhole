// The control folder's commands (`WORMHOLE_DEVCTL`): the renderer's as before, plus the window's.

import AppKit
import Foundation
import TerminalMetal
import WorkspaceModel

extension AppController {
    func run(command line: String) {
        let parts = line.split(separator: " ", maxSplits: 1).map(String.init)
        let argument = parts.count > 1 ? parts[1] : ""
        switch parts.first ?? "" {
        case "sidebar": store.send(.toggleSidebar); reply("sidebar")
        case "panel": store.send(.togglePanel); reply("panel")
        case "palette": showPalette(); reply("palette")
        case "shortcuts": showShortcuts(); reply("shortcuts")
        case "new": newTerminal(); reply("new")
        case "close": closeTerminal(); reply("close")
        case "select":
            guard let id = Int(argument) else { return reply("error: select needs a terminal id") }
            store.send(.select(id))
            reply("select \(id)")
        case "tab":
            guard let tab = PanelTab(rawValue: argument) else { return reply("error: tab is notes, changes or history") }
            store.send(.showPanelTab(tab))
            reply("tab \(argument)")
        case "quit":
            reply("quit")
            NSApp.terminate(nil)
        default:
            guard let view = selectedView else { return reply("error: no terminal is shown") }
            run(command: parts.first ?? "", argument: argument, on: view)
        }
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

    private func reply(_ text: String) { devctl?.append(text) }

    private func state(_ view: MetalTerminalView) -> String {
        let renderer = view.renderer
        let grid = renderer.gridSize, cell = renderer.cellPixels, stats = view.statistics, counters = renderer.counters
        let shown = store.state
        return "grid=\(grid.cols)x\(grid.rows) cell=\(cell.width)x\(cell.height)px scale=\(renderer.scale) "
            + "frames_drawn=\(stats.framesDrawn) ticks=\(stats.ticks) wakes=\(stats.wakes) pauses=\(stats.pauses) "
            + "busy_skips=\(stats.busySkips) blink_fires=\(stats.blinkFires) link=\(view.isLinkRunning ? "running" : "paused") "
            + "words_shaped=\(counters.wordsShaped) word_hits=\(counters.wordHits) glyphs_rasterised=\(counters.glyphsRasterised) "
            + "atlas_resets=\(counters.atlasResets) "
            + "terminals=\(shown.terminals.count) selected=\(shown.selected.map(String.init) ?? "none") "
            + "sidebar=\(shown.sidebarVisible) panel=\(shown.panelVisible) tab=\(shown.panelTab.rawValue) "
            + "window=\(window?.windowNumber ?? 0)"
    }
}

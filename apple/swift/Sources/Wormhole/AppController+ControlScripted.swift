// What a script does where a person would use the pointer, the pasteboard or a panel, which a `--background` instance
// cannot be given: the terminal's paste and drop from a pasteboard of their own (never the system's), a link under a
// held-Command pointer, the print job saved as a PDF, and the sheet a menu item opened (Export Text As, Print) answered. The decisions are the real
// ones; only the hand is the script's.

import AppKit
import TerminalMetal

extension AppController {
    func runScripted(command: String, argument: String) -> Bool {
        switch command {
        case "paste", "drop-files":
            guard let view = selectedView else { reply("error: no terminal is shown"); return true }
            let board = NSPasteboard.withUniqueName()
            if command == "paste" {
                board.setString(
                    argument.replacingOccurrences(of: "\\n", with: "\n").replacingOccurrences(of: "\\e", with: "\u{1b}"), forType: .string)
            } else {
                board.writeObjects(argument.split(separator: "|").map { URL(fileURLWithPath: String($0)) as NSURL })
            }
            reply("\(command) \(view.insert(from: board) ? "taken" : "refused")")
        case "link", "link-click":
            let cell = argument.split(separator: " ").compactMap { Int($0) }
            guard let view = selectedView, cell.count == 2 else { reply("error: usage \(command) <row> <column>"); return true }
            reply(view.scriptedLink(row: cell[0], column: cell[1], click: command == "link-click"))
        case "probes", "fold-toggle":
            // What Probes put on the screen, read from the last frame: the status items, the marker rows, and (`fold-toggle`) a click
            // on the first marker, as the person's would be.
            guard let view = selectedView else { reply("error: no terminal is shown"); return true }
            if command == "fold-toggle", let first = view.foldMarkers.first { _ = view.toggleFold(first.marker.id) }
            let markers = view.foldMarkers.map { "\($0.row): \($0.marker.label) [\($0.marker.isOpen ? "open" : "closed")] <\($0.marker.spoken)>" }
            reply("status \(look.probeItems.map { "\($0.text) (\($0.tooltip))" }) markers \(markers)")
        case "print-pdf":
            // The print job without its panel (the panel itself is opened by `action print` and read with `sheet info`).
            reply("print-pdf \(printTerminal(pdf: URL(fileURLWithPath: argument)) ? argument : "nothing to print")")
        case "sheet":
            guard let sheet = window?.attachedSheet else { reply("no sheet"); return true }
            let words = argument.split(separator: " ", maxSplits: 1).map(String.init)
            guard let answer = words.first, ["ok", "cancel", "info"].contains(answer) else {
                reply("error: usage sheet ok [path] | cancel | info")
                return true
            }
            if let panel = sheet as? NSSavePanel, words.count > 1 {
                let url = URL(fileURLWithPath: words[1])
                panel.directoryURL = url.deletingLastPathComponent()
                panel.nameFieldStringValue = url.lastPathComponent
            }
            reply("sheet \(type(of: sheet)) title=\(sheet.title) name=\((sheet as? NSSavePanel)?.nameFieldStringValue ?? "-") answer=\(answer)")
            if answer != "info" { window?.endSheet(sheet, returnCode: answer == "ok" ? .OK : .cancel) }
        default: return false
        }
        return true
    }
}

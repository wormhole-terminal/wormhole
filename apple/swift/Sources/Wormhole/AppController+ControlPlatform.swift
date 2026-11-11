// The control folder's commands for the platform surfaces that a test instance never puts on the screen, so a driver can reach
// everything up to the last system call:
//   `question` says what a test instance holds instead of a sheet (`PendingQuestions`); `question confirm|cancel|also [n]|<place>`
//   presses a button. Only the control folder does this; the control socket (`wh`) cannot answer a question.
//   `ambient-run <row>` runs what choosing a row of the menu bar extra, the Dock or the quick panel does.
//   `hotkey` says what the global key is (spec, action, held off, error, what the menus show); `hotkey-press` presses it (what
//   the key would do is said, and done); `hotkey-pause on|off` is the Settings recorder holding it off.

import AuxWindows
import DesignSystem
import PlatformModel
import PlatformUI

extension AppController {
    /// For the state dump: the question held, when this is a test instance.
    var pendingQuestionState: String { PendingQuestions.current.map { " question=\($0.brief)" } ?? "" }

    func runPlatform(command: String, argument: String) -> Bool {
        switch command {
        case "question":
            guard let pending = PendingQuestions.current else { reply("error: this instance shows its questions as sheets"); return true }
            reply(argument.isEmpty ? pending.summary : pending.answer(argument))
        case "row-menu":
            // ⌃↩ on a sidebar row, as the list's key handler calls it (`row-menu [terminal]`, the selected one by default): the
            // menu is kept unshown for `menu` and `menu-pick`. The keys themselves never reach a SwiftUI list from outside.
            guard let id = argument.isEmpty ? hub.state.selected : Int(argument), let session = session(holding: id) else {
                reply("error: row-menu [terminal]")
                return true
            }
            openRowMenu(id, at: .zero, in: session)
            reply("row-menu \(id)\n" + (injector.menu.map(Self.rows) ?? "no menu"))
        case "open-folder":
            // Answers Open folder… (⌘O, the palette, the status item) as the system panel's choice would.
            guard let pending = pendingFolder else { reply("error: no Open folder… is waiting"); return true }
            pendingFolder = nil
            pending(argument)
            reply("open-folder \(argument)")
        case "pasteboard":
            // What the instance "copied": its clipboard is a file in its data folder, never the person's (`DesignSystem.Pasteboard`).
            reply(Pasteboard.isDiverted ? (Pasteboard.text().map { "pasteboard \($0.debugDescription)" } ?? "pasteboard empty") : "error: this instance uses the system clipboard")
        case "launches":
            // What the instance would have opened (an address, Finder, an editor at a line): a file in its data folder, never launched.
            reply(Launcher.log().map { "launches\n" + $0 } ?? "error: this instance launches for real")
        case "pasteboard-set":
            guard Pasteboard.isDiverted else { reply("error: this instance uses the system clipboard"); return true }
            Pasteboard.setText(argument.replacingOccurrences(of: "\\n", with: "\n").replacingOccurrences(of: "\\e", with: "\u{1b}"))
            reply("pasteboard-set")
        case "hotkey":
            guard let key = hotkey else { reply("error: no hotkey in this instance"); return true }
            reply("hotkey spec=\(key.applied ?? "-") action=\(key.action) paused=\(key.paused) error=\(key.error ?? "-") shown=\(key.shown ?? "-")")
        case "hotkey-press":
            guard let key = hotkey else { reply("error: no hotkey in this instance"); return true }
            key.press()
            reply("hotkey-press \(key.action)")
        case "hotkey-pause":
            hotkey?.pause(argument == "on")
            reply("hotkey-pause \(argument)")
        case "ambient-run":
            // A row of the menu bar extra, the Dock's menu or the quick panel, run as choosing it would (a test instance makes none).
            let words = argument.split(separator: " ", maxSplits: 1).map(String.init)
            let rest = words.count > 1 ? words[1] : ""
            let parts = rest.split(separator: " ", maxSplits: 1).map(String.init)
            let command: AmbientCommand? =
                switch words.first ?? "" {
                case "new-terminal": .newTerminal
                case "new-window": .newWindow
                case "open-tab": Int64(rest).map { .openTab($0) }
                case "reply": parts.count == 2 ? Int64(parts[0]).map { .reply(tab: $0, text: parts[1]) } : nil
                case "jump-to-waiting": .jumpToWaiting
                case "open-project": rest.isEmpty ? nil : .openProject(path: rest)
                case "run-action": parts.count == 2 ? .runAction(root: parts[0], id: parts[1]) : nil
                default: nil
                }
            guard let command else { reply("error: ambient-run new-terminal|new-window|open-tab <id>|reply <id> <text>|jump-to-waiting|open-project <path>|run-action <root> <id>"); return true }
            run(ambient: command)
            reply("ambient-run \(argument)")
        default: return false
        }
        return true
    }
}

// text-file: a test driver's replies on the control folder, not interface text
// The control folder's right panel read-outs (`WORMHOLE_DEVCTL`):
//   `changes-menu <n|path>`     right-click the n-th file of `changes` (or the file by path): its menu, kept for `menu-pick`
//   `history-menu <n|details>`  right-click the n-th commit of the log, or the open commit's details block
//   `files-row-menu <path>`     right-click a Files row by its path from the tree's root
//   `files-viewer [from] [count]`  what the Files viewer shows (`PanelUI.FilesViewerReadout`)
//   `panel-kept`                what the Files, Changes and History stores keep per terminal (`PanelUI.PanelKeptReadout`)
//   `diff-header`               what the open diff's header says (`PanelUI.DiffReadout`)
//   `diff-step next|prev`       its arrows
//   `diff-spans [from] [count]` the diff's rows with their colour runs by token
// A menu is found as `right-click` finds it, at the row's own frame (`PanelRowFrames`): the row must be on screen, so show
// its tab first (`tab changes`) and ask again until it is laid out. Replies: `menu:` then `AppController.rows(of:)`'s lines.

import AppKit
import PanelModel
import PanelUI

extension AppController {
    /// False when `command` is not one of these.
    func runPanelReadouts(command: String, argument: String) -> Bool {
        let argument = argument.trimmingCharacters(in: .whitespaces)
        let panel = hub.panel
        switch command {
        case "changes-menu":
            let files = panel.changes.files
            let path = Int(argument).flatMap { files.indices.contains($0 - 1) ? files[$0 - 1].path : nil } ?? argument
            reply(rightClickRow("file:\(path)"))
        case "history-menu":
            if argument == "details" { reply(rightClickRow("details")); return true }
            let commits = panel.history.model.commits
            guard let n = Int(argument), commits.indices.contains(n - 1) else { reply("error: history-menu <n|details> (n of \(commits.count) commits)"); return true }
            reply(rightClickRow("commit:\(commits[n - 1].id)"))
        case "files-row-menu":
            guard !argument.isEmpty else { reply("error: files-row-menu <path from the tree's root>"); return true }
            reply(rightClickRow("entry:\(argument)"))
        case "files-viewer":
            let numbers = argument.split(separator: " ").compactMap { Int($0) }
            reply(FilesViewerReadout.text(panel.files, from: numbers.first ?? 0, count: numbers.count > 1 ? numbers[1] : 20))
        case "panel-kept": reply(PanelKeptReadout.text(panel))
        case "diff-header": reply(DiffReadout.header(panel))
        case "diff-step": reply(DiffReadout.step(panel, argument).map { "error: \($0)" } ?? "diff-step \(argument)")
        case "diff-spans":
            let numbers = argument.split(separator: " ").compactMap { Int($0) }
            reply(DiffReadout.spans(panel, from: numbers.first ?? 0, count: numbers.count > 1 ? numbers[1] : 60))
        default: return false
        }
        return true
    }

    /// The menu a right-click on the row recorded under `id` opens, kept unshown for `menu` and `menu-pick`.
    private func rightClickRow(_ id: String) -> String {
        guard let window else { return "error: no window" }
        guard let frame = PanelRowFrames.frame(id) else { return "error: no row \(id) on screen (its tab shown, its list laid out?)" }
        let point = injector.location(Double(frame.midX), Double(frame.midY), in: window)
        guard let menu = injector.contextMenu(point, in: window) else { return "error: the row \(id) gave no menu" }
        return "menu:\n" + Self.rows(of: menu)
    }
}

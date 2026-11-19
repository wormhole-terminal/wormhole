// The control folder's Files tab commands (`WORMHOLE_DEVCTL`), GPUI's `devctl/files.rs`: `files` (the tab as text),
// `files-root <path>`, `files-follow`, `files-pin <path>`, `files-rename <path> <new name>`. They call what the tab's own
// buttons and menu call (`FilesStore.choose`, `followTerminal`, `setPinned`, `startRename`, `commitRename`). The ones the
// Swift control line had already (`files-open`, `files-expand`, `files-rename-start`) are in `AppController+ControlPanel`.

import Foundation
import PanelModel
import PanelUI
import WorkspaceModel

extension AppController {
    /// False when `command` is not one of these.
    func runFiles(command: String, argument: String) -> Bool {
        let argument = argument.trimmingCharacters(in: .whitespaces)
        switch command {
        case "files", "files-root", "files-follow", "files-pin", "files-rename": break
        default: return false
        }
        hub.send(.showPanelTab(.files))
        let files = hub.panel.files
        switch command {
        case "files": reply(describe(files))
        case "files-root":
            // As if chosen in the path menu: a folder anywhere, so an absolute path.
            guard argument.hasPrefix("/") else { reply("error: expected an absolute path, got \(argument)"); return true }
            files.choose(argument)
            reply("files-root \(argument)")
        case "files-follow":
            files.followTerminal()
            reply("files-follow")
        case "files-pin":
            // Pins the folder, or unpins it; the tab names a folder below its root, so the path is made relative to it.
            guard let root = files.root, argument.hasPrefix("/") else { reply("error: expected an absolute path, got \(argument)"); return true }
            let prefix = root == "/" ? "/" : root + "/"
            let relative: String? = argument == root ? "" : argument.hasPrefix(prefix) ? String(argument.dropFirst(prefix.count)) : nil
            guard let relative else { reply("error: \(argument) is not below the tree's root \(root)"); return true }
            files.setPinned(relative, !files.isPinned(relative))
            reply("files-pin \(argument)")
        default:
            // `files-rename <relative path> <new name>`: the path has no space in it, the name is the rest of the line.
            let words = argument.split(separator: " ", maxSplits: 1).map(String.init)
            guard words.count == 2, let row = files.rows.first(where: { $0.path == words[0] }) else { reply("error: files-rename <path> <new name>"); return true }
            files.startRename(row)
            files.commitRename(to: words[1])
            reply("files-rename \(argument)")
        }
        return true
    }

    /// The tab as text, GPUI's shape: the folder and how it was chosen, the pins, the viewer, a rename, then the rows
    /// indented by depth (`v` an open folder, `>` a closed one, `~` a link, `*` the selected row).
    private func describe(_ files: FilesStore) -> String {
        guard let root = files.root else { return "files: no folder" }
        var lines = ["files: \(root) (\(files.chosen != nil ? "chosen" : "following")) pinned=\(files.pins.isEmpty ? "-" : files.pins.joined(separator: ","))"]
        if let viewing = files.viewing { lines.append("viewer: \(viewing)") }
        if let renaming = files.renaming { lines.append("renaming: \(renaming)") }
        for row in files.rows {
            let mark = files.selected == row.path && row.isEntry ? "*" : " "
            let indent = String(repeating: "  ", count: row.depth + 1)
            let body: String = switch row.kind {
            case .folder(let open): (open ? "v " : "> ") + row.label
            case .file: "  " + row.label
            case .link: "~ " + row.label
            case .more(let count, let atLeast): "+ \(count)\(atLeast ? "+" : "") more"
            case .reading: "… reading"
            case .empty: "(empty)"
            case .unreadable(let failure): "(can't read: \(failure))"
            }
            lines.append(mark + indent + body)
        }
        return lines.joined(separator: "\n")
    }
}

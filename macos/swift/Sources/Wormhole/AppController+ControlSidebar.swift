// The control folder's sidebar and folder picker commands (`WORMHOLE_DEVCTL`), GPUI's `devctl/sidebar.rs` and
// `folder_picker/devctl.rs`: `group-ask use|later`, `rename-edit <row>`, `folder-color [path] <name|#rrggbb|none>` and
// `folder-picker open|close|type|tab|right|left|accept-ghost|up|down|pageup|pagedown|enter|pick|forget|click|crumb|scroll|state`.
// Each does what the key or button does: the sidebar's own request, the Project tab's backend, the picker driver's key.

import AppKit
import AuxModel
import AuxWindows
import Foundation
import PanelModel
import PlatformModel
import PlatformUI
import SidebarUI
import WorkspaceModel

extension AppController {
    /// False when `command` is not one of these.
    func runSidebar(command: String, argument: String) async -> Bool {
        let argument = argument.trimmingCharacters(in: .whitespaces)
        switch command {
        case "group-ask":
            // The question a terminal opening in a group without a folder puts up (held for this, as every question of a test instance is):
            // `use`, `use only` (the box ticked: only this folder's terminals stay) or `later` (Not Now; there is no Cancel). No argument:
            // the folder, the group and how many terminals would leave.
            guard let pending = PendingQuestions.current else { reply("error: this instance shows its questions as sheets"); return true }
            switch argument {
            case "": reply(pending.summary)
            case "use": reply(pending.answer("confirm"))
            case "use only": reply(pending.answer("also 1"))
            case "later": reply(pending.answer("also 2"))
            default: reply("error: group-ask takes use, use only or later")
            }
        case "rename-edit":
            // The terminal's name becomes a field, as Rename… in its menu makes it.
            let all = hub.state.all
            guard let n = Int(argument), all.indices.contains(n - 1) else { reply("error: no row \(argument)"); return true }
            hub.sidebarLink.ask(.rename(.terminal(all[n - 1].id)))
            reply("rename-edit \(n)")
        case "folder-color": await folderColor(argument)
        case "folder-picker": await folderPicker(argument)
        default: return false
        }
        return true
    }

    /// A test instance holds the group-folder question for `group-ask` like every other question. The box is a button of its own
    /// (a held question has no checkbox to tick): Use Folder, Use Folder and keep only this folder's terminals, Not Now.
    func installGroupAsker() {
        guard PendingQuestions.current != nil else { return }
        GroupFolderQuestion.asker = { offer in
            let words = GroupFolderQuestion.words(offer)
            let question = Question(title: words.title, detail: words.detail, confirm: words.use, alsoButtons: [words.only, words.decline])
            switch await QuestionSheet(question).ask(in: nil) {
            case .confirm: return .use(onlyInFolder: false)
            case .also(let place): return place == 0 ? .use(onlyInFolder: true) : .decline
            case .cancel: return .decline
            }
        }
    }

    /// `folder-color [path] <name|#rrggbb|none>`: the folder (the active terminal's without a path; `~/` is home) coloured or not.
    private func folderColor(_ argument: String) async {
        let words = argument.split(separator: " ").map(String.init)
        guard let name = words.last else { return reply("error: folder-color [path] <name|#rrggbb|none>") }
        let path = words.count > 1 ? HomeFolder.expanding(words.dropLast().joined(separator: " ")) : hub.panel.project.target
        guard let folder = path, folder.hasPrefix("/") else { return reply("error: expected an absolute path, got \(path ?? "none")") }
        let backend = front.panelBackends.project
        if name == "none" {
            await backend.setFolderColor(nil, of: folder)
        } else if let color = FolderColor(rawValue: name) {
            await backend.setFolderColor(color, of: folder)
        } else if name.hasPrefix("#"), name.count == 7, name.dropFirst().allSatisfy(\.isHexDigit) {
            await backend.setFolderColor(hex: name, of: folder)
        } else {
            return reply("error: expected none, a #rrggbb colour or one of \(FolderColor.allCases.map(\.rawValue).joined(separator: ", ")), got \(name)")
        }
        hub.panel.project.reload()
        reply("\(name == "none" ? "no colour" : name) \(folder)")
    }

    private func folderPicker(_ argument: String) async {
        let (word, rest) = argument.split(separator: " ", maxSplits: 1).map(String.init).first.map { word in
            (word, String(argument.dropFirst(word.count)).trimmingCharacters(in: .whitespaces))
        } ?? ("", "")
        switch word {
        case "open":
            guard flagsOn(front).contains("folder_picker") else { return reply("error: the Folder picker flag is off: set feature_flags={\"folder_picker\":true}") }
            if folderPicking.driver == nil { folderPicking.open() }
            return reply("folder-picker open")
        case "close":
            folderPicking.close()
            return reply("folder-picker close")
        case "type", "tab", "right", "left", "accept-ghost", "up", "down", "pageup", "pagedown", "enter", "pick", "forget", "click", "crumb", "scroll", "state": break
        default: return reply("error: usage: folder-picker open|close|type <text>|tab|right [mid]|left [start|mid]|accept-ghost|up|down|pageup|pagedown|enter|pick|forget|click <line>|crumb <n>|scroll <lines>|state")
        }
        guard let driver = folderPicking.driver else { return reply("error: the folder picker is not open") }
        let text = driver.model.text
        let end = PickerField(text: text), middle = PickerField(text: text, caret: text.count / 2)
        switch word {
        case "type": driver.typed(rest)
        case "tab": _ = driver.key(.tab)
        case "accept-ghost": driver.field = end; _ = driver.key(.right)
        case "right": driver.field = rest == "mid" ? middle : end; _ = driver.key(.right)
        case "left": driver.field = rest == "start" ? PickerField(text: text, caret: 0) : rest == "mid" ? middle : end; _ = driver.key(.left)
        case "up": _ = driver.key(.up)
        case "down": _ = driver.key(.down)
        case "pageup": _ = driver.key(.pageUp)
        case "pagedown": _ = driver.key(.pageDown)
        case "enter", "pick": _ = driver.key(.enter)
        case "forget": _ = driver.key(.forget)
        case "click":
            guard let line = Int(rest), driver.model.lines.indices.contains(line), case .row(let row) = driver.model.lines[line] else { return reply("error: click needs the number of a row's line") }
            driver.click(row.key)
        case "crumb":
            guard let n = Int(rest) else { return reply("error: crumb needs a number") }
            driver.click(crumb: n)
        case "scroll":
            // The panel shows its rows without a scroll position of its own: nothing to move.
            return reply("folder-picker scroll: the list is shown in full (at most \(9) rows)")
        default: return reply(pickerSummary(driver))
        }
        reply("folder-picker \(argument)")
    }

    /// `folder-picker state`, GPUI's shape: the text, the ghost, whether it is busy, the crumbs, the facts, the lines, then the panel.
    private func pickerSummary(_ driver: FolderPickerDriver) -> String {
        let model = driver.model, facts = model.facts
        var out = "typed \(model.text.debugDescription)\nghost \((model.ghost ?? "").debugDescription)\nbusy \(driver.isBusy)\n"
            + "crumbs \(model.answer.crumbs.map(\.label).joined(separator: " > "))\n"
            + "facts up \(facts.up) descends \(facts.descends) ghost \(facts.ghost) forgettable \(facts.forgettable)\nlines \(model.lines.count) shown \(model.answer.rows.count)\n"
        for line in model.lines {
            switch line {
            case .header(let title): out += "  # \(title)\n"
            case .row(let row):
                var text = (model.selectedKey == row.key ? "> " : "  ") + "\(row.label) [\(row.icon)]"
                if let badge = row.badge { text += " badge \(badge.debugDescription)" }
                if row.chevron { text += " chevron" }
                if let detail = row.detail { text += " detail \(detail.debugDescription)" }
                if row.forgettable { text += " forgettable" }
                out += text + " insert \((row.insert ?? "").debugDescription) descend \((row.descend ?? "").debugDescription) key \(row.key.debugDescription)\n"
            case .note(let note): out += "  note \(note)\n"
            }
        }
        for note in model.answer.notes { out += "  note \(note)\n" }
        if let frame = folderPicking.panelFrame { out += "panel x \(Int(frame.minX)) y \(Int(frame.minY)) w \(Int(frame.width)) h \(Int(frame.height))\n" }
        return out
    }
}

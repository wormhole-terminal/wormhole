// The title row's controls that open a SwiftUI popover or menu, which a script cannot press: each command lists the items as
// the view builds them and picks one through the same call the view's button makes. What leaves the app (an editor, the browser,
// Finder) is recorded by `ExternalOpen` in a test instance and read with `launches` (`ControlPlatform`).
//   `open-in` lists the Open in… button and its menu, `open-in click` presses the button, `open-in <n|name>` an item;
//   `path-card [<n|name>]`, `ports [<n>]` (the chips, then the "+n" menu's) and `pull-request` likewise.

import DesignSystem
import Foundation
import PanesModel
import PanesUI

extension AppController {
    func runTitle(command: String, argument: String) -> Bool {
        switch command {
        case "open-in":
            guard let menu = hub.selectedOpenIn else { reply("error: the selected terminal has no folder") ; return true }
            if argument == "click" {
                menu.click()
                reply("open-in click \(menu.choice.app.name)")
            } else if let item = pick(argument, from: menu.items, title: { $0.app.name }) {
                menu.pick(item.app)
                reply("open-in \(item.app.name)")
            } else if argument.isEmpty {
                let missing = menu.choice.missing.map { " missing=\($0)" } ?? ""
                reply("open-in folder=\(menu.folder) button=\(menu.choice.app.name)\(missing)\n" + Self.numbered(menu.items.map { ($0.checked ? "[x] " : "[ ] ") + $0.app.name }))
            } else {
                reply("error: open-in [click|<n>|<name>]")
            }
        case "path-card":
            guard let actions = hub.selectedTitleActions, let folder = hub.state.selectedTerminal?.folder else { reply("error: the selected terminal has no folder"); return true }
            if argument.isEmpty {
                reply("path-card \(folder)\n" + Self.numbered(PathCardItem.allCases.map { $0.title + ($0.isOn(actions) ? "" : " [off]") }))
            } else if let item = pick(argument, from: PathCardItem.allCases, title: \.title) {
                guard item.isOn(actions) else { reply("error: \(item.title) is off"); return true }
                item.perform(actions)
                reply("path-card \(item.title)")
            } else {
                reply("error: path-card [<n>|<name>]")
            }
        case "ports":
            let ports = hub.state.title?.ports ?? []
            let chips = Ports.chips(ports).shown.count
            if argument.isEmpty {
                reply("ports chips=\(chips) more=\(ports.count - chips)\n" + Self.numbered(ports.enumerated().map { "\(Ports.label($1))" + ($0 < chips ? "" : " (+n menu)") }))
            } else if let port = pick(argument, from: ports, title: { Ports.label($0) }) {
                TitleClicks.openPort(port)
                reply("ports \(Ports.label(port))")
            } else {
                reply("error: ports [<n>|<name>]")
            }
        case "pull-request":
            guard let pr = hub.state.title?.pullRequest else { reply("error: no pull request"); return true }
            TitleClicks.openPullRequest(pr)
            reply("pull-request \(pr.label) \(pr.words)")
        default: return false
        }
        return true
    }

    /// The item an argument names: its 1-based number, or its title.
    private func pick<T>(_ argument: String, from items: [T], title: (T) -> String) -> T? {
        guard !argument.isEmpty else { return nil }
        if let n = Int(argument) { return items.indices.contains(n - 1) ? items[n - 1] : nil }
        return items.first { title($0) == argument }
    }

    private static func numbered(_ lines: [String]) -> String {
        lines.enumerated().map { "\($0 + 1) \($1)" }.joined(separator: "\n")
    }
}

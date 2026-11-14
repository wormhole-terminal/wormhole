// text-file: a test driver's replies on the control folder, not interface text
// `settings-press <id> [menu]`: a click on a Settings control by the id it records its frame under (`SettingsUI.RowFrames`:
// a row's control by the row's id, a page's buttons by their own: `changed.copy`, `import-theme`, `open-studio`, `reset-tips`,
// `open-in.choose`, `zen.leave`, `theme-card.<value>`, …; `settings-rows` lists them), made the way `click` makes one, at
// the frame's centre in the Settings window (`at <x> [y]`: at those fractions of the frame). With `menu`, a right-click there
// instead, its menu kept for `menu-pick`.
// Replies `settings-press <id>: <what the press and release said>`, or `menu:` and the menu's lines.

import AppKit
import SettingsUI

extension AppController {
    /// False when `command` is not `settings-press`.
    func runSettingsPress(command: String, argument: String) async -> Bool {
        guard command == "settings-press" else { return false }
        let words = argument.split(separator: " ").map(String.init)
        guard let id = words.first, let window = settingsController?.window, let row = RowFrames.all.first(where: { $0.id == id }) else {
            reply("error: settings-press <id> [menu | at <x> [y]] (a control on show: `settings-rows`)")
            return true
        }
        // `at <x> [y]`: the click at those fractions of the control's frame instead of its centre, where a person aims at one
        // part of it: a segment of a segmented control (as wide as its words), a stepper's up or down arrow.
        var at = (x: 0.5, y: 0.5)
        if words.dropFirst().first == "at" {
            guard words.count >= 3, let x = Double(words[2]), (0...1).contains(x) else {
                reply("error: settings-press <id> at <x 0…1> [y 0…1]")
                return true
            }
            at = (x, words.count > 3 ? min(max(Double(words[3]) ?? 0.5, 0), 1) : 0.5)
        }
        let point = injector.location(
            Double(row.frame.minX) + Double(row.frame.width) * at.x, Double(row.frame.minY) + Double(row.frame.height) * at.y, in: window)
        if words.dropFirst().first == "menu" {
            guard let menu = injector.contextMenu(point, in: window) else { reply("error: \(id) gave no menu"); return true }
            reply("menu:\n" + Self.rows(of: menu))
            return true
        }
        var said = [injector.press(point, in: window, clicks: 1, flags: [])]
        if injector.isPressed { said.append(injector.release(point, in: window, clicks: 1, flags: [])) }
        // SwiftUI acts on the release on the next turn of the run loop.
        await pause(60)
        reply("settings-press \(id): " + said.joined(separator: ", "))
        return true
    }
}

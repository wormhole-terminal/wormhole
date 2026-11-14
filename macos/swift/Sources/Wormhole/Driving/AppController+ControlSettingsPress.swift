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
        guard let id = words.first, let window = settingsController?.window, let frame = await settingsFrame(id, in: window) else {
            reply("error: settings-press <id> [menu | at <x> [y]] (a control on show: `settings-rows`)")
            return true
        }
        let row = (id: id, frame: frame)
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

    /// Where the Settings control recorded under `id` is, in the window's points from its top left, once it is in sight: a
    /// control below or above what the page shows is scrolled to first, by the wheel, as a person does before pressing it.
    /// On a small display the window is shorter than a page, and a press at the recorded frame landed outside the window.
    /// Nil for an id that is not on the page.
    func settingsFrame(_ id: String, in window: NSWindow) async -> CGRect? {
        func recorded() -> CGRect? { RowFrames.all.first { $0.id == id }?.frame }
        guard let content = window.contentView else { return recorded() }
        // The title bar's strip at the top and a row's height at the bottom are left clear.
        let sight = 60...(Double(content.bounds.height) - 30)
        for _ in 0..<80 {
            guard let frame = recorded() else { return nil }
            let off = Double(frame.midY) < sight.lowerBound ? Double(frame.midY) - sight.lowerBound : Double(frame.midY) - sight.upperBound
            if sight.contains(Double(frame.midY)) { return frame }
            // The wheel over the page's own column, a step at a time: the frame is recorded again as the page moves.
            let over = injector.location(Double(frame.midX), Double(content.bounds.height) / 2, in: window)
            _ = injector.scroll(over, in: window, dy: max(-120, min(120, off)))
            await pause(16)
        }
        return recorded()
    }
}

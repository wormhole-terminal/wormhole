// text-file: a test driver's replies on the control folder, not interface text
// The control folder's `inspect` (GPUI's devctl has `inspect`, and `hover x y` / `click x y` highlight and pin while it is
// on): inspect mode from a script. `inspect` toggles it, `inspect on|off` sets it, `inspect X Y [levels]` pins the element
// at a point and prints its note and its parents, `inspect parent` walks out one, `inspect pick` picks again and
// `inspect copy` is the panel's Copy (a test instance's clipboard is its own file: read it with `pasteboard`).

import AppKit
import DesignSystem

extension AppController {
    func runInspect(command: String, argument: String) async -> Bool {
        let inspector = Inspector.shared
        let words = argument.split(separator: " ").map(String.init)
        let numbers = words.compactMap(Double.init)
        switch (command, words.first) {
        case ("hover", _) where inspector.picking && numbers.count >= 2, ("click", _) where inspector.picking && numbers.count >= 2:
            guard let target = drivenWindow ?? window else { return false }
            let point = injector.location(numbers[0], numbers[1], in: target)
            if command == "click" { inspector.pin(at: point, in: target) } else { inspector.point(at: point, in: target) }
            reply("\(command) \(argument): inspect\n" + inspected(levels: 1, in: target))
        case ("inspect", nil):
            inspector.toggle(beside: window)
            await settle(inspector)
            reply("inspect \(inspector.isOpen ? "on" : "off")")
        case ("inspect", "on"), ("inspect", "off"):
            if words[0] == "on" { inspector.open(beside: window) } else { inspector.close() }
            await settle(inspector)
            reply("inspect \(words[0])")
        case ("inspect", "pick"):
            inspector.startPicking()
            reply(inspector.picking ? "inspect picking" : "error: inspect mode is off (inspect on)")
        case ("inspect", "parent"):
            reply(inspector.walkOut() ? inspector.note ?? "" : "error: no parent (the outermost marked element, or nothing picked)")
        case ("inspect", "copy"):
            reply(inspector.copy().map { "inspect copied\n\($0)" } ?? "error: nothing is picked (inspect X Y)")
        case ("inspect", _):
            guard numbers.count >= 2, let target = drivenWindow ?? window else { reply("error: inspect [on|off|pick|parent|copy|X Y [levels]]"); return true }
            // Asked of a window that was not being inspected: the elements report themselves once they are laid out again.
            if !inspector.isOpen {
                inspector.open(beside: window)
                await settle(inspector)
            }
            inspector.pin(at: injector.location(numbers[0], numbers[1], in: target), in: target)
            reply(inspected(levels: numbers.count > 2 ? Int(numbers[2]) : 3, in: target))
        default: return false
        }
        return true
    }

    /// Two frames for the marked elements to put their probes in (or take them out) after inspect mode changed.
    private func settle(_ inspector: Inspector) async {
        await pause(32)
        NSApp.windows.forEach { $0.contentView?.layoutSubtreeIfNeeded() }
    }

    /// The pinned element's note, then `levels - 1` of its parents, one a line: where each is built, its id and its frame
    /// from the window content's top left, as `bounds` prints points.
    private func inspected(levels: Int, in window: NSWindow) -> String {
        let inspector = Inspector.shared
        guard let note = inspector.note else { return "inspect: no marked element there" }
        let height = window.contentView?.bounds.height ?? window.frame.height
        let parents = inspector.chain.dropFirst(inspector.level + 1).prefix(max(0, levels - 1)).map { element in
            let frame = element.frame
            return String(
                format: "parent %@  %@  %.0f,%.0f %.0f×%.0f", element.site.location, element.site.id, frame.minX, height - frame.maxY, frame.width,
                frame.height)
        }
        return ([note] + parents).joined(separator: "\n")
    }
}

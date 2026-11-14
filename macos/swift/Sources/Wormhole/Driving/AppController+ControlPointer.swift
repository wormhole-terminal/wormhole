// The control folder's pointer (`WORMHOLE_DEVCTL`), after GPUI's devctl: click, double-click, right-click, hover, the
// button held and moved, a drag in steps, the wheel, and the menu a right-click found. Points are from the window
// content's top left, in points. `Injector` says how the events reach the views without the window server.

import AppKit

extension AppController {
    /// False when `command` is not one of the pointer's; a drag and the wheel go in steps, a frame apart.
    func runPointer(command: String, argument: String) async -> Bool {
        let words = argument.split(separator: " ").map(String.init)
        let numbers = words.prefix { Double($0) != nil }.compactMap(Double.init)
        let flags = words.dropFirst(numbers.count).first.map(Self.modifierFlags) ?? []
        guard let window = pointerWindow else { return false }
        func at(_ i: Int) -> NSPoint { injector.location(numbers[i], numbers[i + 1], in: window) }
        switch (command, numbers.count) {
        case ("click", 2...), ("double-click", 2...), ("triple-click", 2...):
            var said = [String]()
            for clicks in 1...(["click": 1, "double-click": 2][command] ?? 3) {
                said.append(injector.press(at(0), in: window, clicks: clicks, flags: flags))
                if injector.isPressed { said.append(injector.release(at(0), in: window, clicks: clicks, flags: flags)) }
                await pause(30)
            }
            reply("\(command) \(argument): " + said.joined(separator: ", "))
        case ("right-click", 2...):
            let menu = injector.contextMenu(at(0), in: window)
            reply(menu.map { "menu:\n" + Self.rows(of: $0) } ?? "right-click \(argument): no menu, the view took the click")
        case ("hover", 2...), ("hover-cmd", 2...):
            reply("\(command) \(argument): " + injector.move(at(0), in: window, flags: command == "hover-cmd" ? .command : flags))
        case ("mouse-down", 2...): reply(injector.press(at(0), in: window, flags: flags, hold: true))
        case ("mouse-move", 2...), ("mouse-drag", 2...): reply(injector.move(at(0), in: window, flags: flags))
        case ("mouse-up", 2...): reply(injector.release(at(0), in: window, flags: flags))
        case ("drag", 4...):
            // `drag x1 y1 x2 y2 [steps]`: pressed, moved in steps with a frame between them, let go.
            let steps = max(1, numbers.count > 4 ? Int(numbers[4]) : 8)
            let from = at(0), to = at(2)
            let path = (1...steps).map { step in
                NSPoint(x: from.x + (to.x - from.x) * CGFloat(step) / CGFloat(steps), y: from.y + (to.y - from.y) * CGFloat(step) / CGFloat(steps))
            }
            let down = injector.press(from, in: window, flags: flags, path: path)
            guard injector.isPressed else { reply("drag \(argument): \(down)"); return true }
            for point in path {
                injector.move(point, in: window, flags: flags)
                await pause(16)
            }
            reply("drag \(argument): \(down), \(steps) steps, " + injector.release(to, in: window, flags: flags))
        case ("scroll", 3...):
            // In steps of at most 20 points, a frame apart, as a trackpad's scroll arrives (a long list's end is reached).
            // An optional fourth number scrolls sideways: `scroll x y dy dx`.
            let total = numbers[2], across = numbers.count > 3 ? numbers[3] : 0
            let count = max(1, Int((max(abs(total), abs(across)) / 20).rounded(.up)))
            var said = ""
            for _ in 0..<count {
                said = injector.scroll(at(0), in: window, dy: total / Double(count), dx: across / Double(count))
                await pause(16)
            }
            reply("scroll \(argument): \(said) in \(count) steps")
        case ("menu", 0):
            reply(injector.menu.map(Self.rows) ?? "no menu")
        case ("menu-pick", _):
            reply(injector.pick(argument))
        case ("menu-close", 0):
            reply(injector.closeMenu() ? "menu-close" : "no menu")
        case ("pointer-window", _):
            injector.windowTitle = argument.isEmpty ? nil : argument
            reply("pointer-window \(pointerWindow?.title ?? "none")")
        default: return false
        }
        return true
    }

    /// The window the pointer acts on: the workspace's in front, or the one `pointer-window <title>` named (Settings, the Studio).
    private var pointerWindow: NSWindow? { drivenWindow ?? window }

    /// The window `pointer-window <title>` named: by its title, or `popover` for the one a popover (the font picker, a row's
    /// description) shows its content in, which has no title.
    var drivenWindow: NSWindow? {
        injector.target ?? injector.windowTitle.flatMap { title in
            NSApp.windows.first { title == "popover" ? String(describing: type(of: $0)).contains("Popover") : $0.title == title }
        }
    }

    /// A menu as GPUI's devctl prints one: `n. title [key] [x] [off]`, `---` for a separator, a submenu's items indented.
    static func rows(of menu: NSMenu) -> String {
        var number = 0
        func lines(_ menu: NSMenu, _ indent: String) -> [String] {
            menu.items.flatMap { item -> [String] in
                if item.isSeparatorItem { return [indent + "---"] }
                if indent.isEmpty { number += 1 }
                let key = item.keyEquivalent.isEmpty ? "" : " [\(item.keyEquivalent)]"
                let marks = (item.state == .on ? " [x]" : "") + (item.isEnabled ? "" : " [off]")
                let head = indent + (indent.isEmpty ? "\(number). " : "- ") + item.title + key + marks
                return [head] + (item.submenu.map { lines($0, indent + "    ") } ?? [])
            }
        }
        return lines(menu, "").joined(separator: "\n")
    }

    /// `cmd`, `shift-alt`, … as the modifier flags a key or a click carries.
    static func modifierFlags(_ spec: String) -> NSEvent.ModifierFlags {
        let names: [String: NSEvent.ModifierFlags] = ["shift": .shift, "cmd": .command, "ctrl": .control, "alt": .option]
        return spec.split(separator: "-").reduce(into: []) { flags, name in names[String(name)].map { flags.insert($0) } }
    }

    /// A moment for the views to act on what was sent, as the time between a hand's steps gives them.
    func pause(_ milliseconds: Int) async {
        // one-shot: the frame between two steps of a scripted pointer or a wait; nothing runs while the instance is idle.
        try? await Task.sleep(for: .milliseconds(milliseconds))
    }
}

// The main menu: what the window's keys are. Targets are explicit so the keys work with any first responder.

import AppKit
import TerminalMetal

@MainActor
enum AppMenu {
    static func make(target: AppController) -> NSMenu {
        let main = NSMenu()
        add(to: main, title: "Wormhole", [
            item("Quit", #selector(NSApplication.terminate(_:)), "q", target: NSApp)
        ])
        add(to: main, title: "File", [
            item("New Terminal", #selector(AppController.newTerminal), "t", target: target),
            item("Close Terminal", #selector(AppController.closeTerminal), "w", target: target),
        ])
        add(to: main, title: "Edit", [
            item("Paste", #selector(MetalTerminalView.paste(_:)), "v", target: nil)
        ])
        add(to: main, title: "View", [
            item("Toggle Sidebar", #selector(AppController.toggleSidebar), "b", target: target),
            item("Toggle Panel", #selector(AppController.togglePanel), "b", [.command, .option], target: target),
        ])
        add(to: main, title: "Terminal", (1...9).map { number in
            let entry = item("Select Terminal \(number)", #selector(AppController.selectNumbered(_:)), "\(number)", target: target)
            entry.tag = number
            return entry
        })
        return main
    }

    private static func add(to menu: NSMenu, title: String, _ items: [NSMenuItem]) {
        let holder = NSMenuItem()
        let submenu = NSMenu(title: title)
        items.forEach(submenu.addItem)
        holder.submenu = submenu
        menu.addItem(holder)
    }

    private static func item(
        _ title: String, _ action: Selector, _ key: String, _ modifiers: NSEvent.ModifierFlags = .command, target: AnyObject?
    ) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: key)
        item.keyEquivalentModifierMask = modifiers
        item.target = target
        return item
    }
}

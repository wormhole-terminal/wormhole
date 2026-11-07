// The main menu: what the window's keys are. Targets are explicit so the keys work with any first responder.

import AppKit
import DesignSystem
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
            item("Command Palette", #selector(AppController.showPalette), "p", [.command, .shift], target: target),
            item("Keyboard Shortcuts", #selector(AppController.showShortcuts), "/", target: target),
            .separator(),
            submenu("Theme", themeItems(target: target)),
            submenu("Appearance", ThemeMode.allCases.map { mode in
                let entry = item(mode.rawValue.capitalized, #selector(AppController.chooseAppearance(_:)), "", target: target)
                entry.representedObject = mode.rawValue
                return entry
            }),
            .separator(),
            item("Status Bar", #selector(AppController.toggleStatusBar), "", target: target),
            item("Column Dividers", #selector(AppController.toggleDividers), "", target: target),
            item("Translucent Window", #selector(AppController.toggleTranslucent), "", target: target),
            item("Window Blur", #selector(AppController.toggleBlur), "", target: target),
            item("Liquid Glass", #selector(AppController.toggleLiquidGlass), "", target: target),
        ])
        add(to: main, title: "Terminal", (1...9).map { number in
            let entry = item("Select Terminal \(number)", #selector(AppController.selectNumbered(_:)), "\(number)", target: target)
            entry.tag = number
            return entry
        })
        return main
    }

    /// Every bundled theme, the dark ones and then the light ones: choosing one shows it.
    private static func themeItems(target: AppController) -> [NSMenuItem] {
        let themes = ThemeLibrary.bundled.themes
        var items: [NSMenuItem] = []
        for appearance in ThemeAppearance.allCases {
            if !items.isEmpty { items.append(.separator()) }
            items += themes.filter { $0.appearance == appearance }.map { theme in
                let entry = item(theme.name, #selector(AppController.chooseTheme(_:)), "", target: target)
                entry.representedObject = theme.id
                return entry
            }
        }
        return items
    }

    private static func submenu(_ title: String, _ items: [NSMenuItem]) -> NSMenuItem {
        let holder = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        let menu = NSMenu(title: title)
        items.forEach(menu.addItem)
        holder.submenu = menu
        return holder
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

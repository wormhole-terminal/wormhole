// How the window looks, from the command line, the View menu and the control folder: the theme, the
// appearance, the status bar and the window's own translucency. All of it is one live value in `ThemeStore`
// and `WindowLook`, so every command does the same as the menu item and nothing is redrawn by hand.

import AppKit
import DesignSystem
import WindowUI

extension AppController {
    var themes: ThemeStore { ThemeStore.shared }

    /// What `--theme`, `--appearance` and `--status-bar` asked for, before the window opens.
    func applyLookOptions() {
        if let id = options.theme { themes.setTheme(id) }
        if let mode = options.appearance { themes.mode = mode }
        look.statusBar = options.statusBar
    }

    @objc func chooseTheme(_ sender: NSMenuItem) {
        if let id = sender.representedObject as? String { themes.setTheme(id) }
    }

    @objc func chooseAppearance(_ sender: NSMenuItem) {
        if let mode = (sender.representedObject as? String).flatMap(ThemeMode.init(rawValue:)) { themes.mode = mode }
    }

    @objc func toggleStatusBar() { look.statusBar.toggle() }
    @objc func toggleTranslucent() { look.opacity = look.opacity < 1 ? 1 : 0.8 }
    @objc func toggleBlur() { look.blur.toggle() }
    @objc func toggleLiquidGlass() { look.liquidGlass.toggle() }
    @objc func toggleDividers() { look.columnDividers.toggle() }

}

extension AppController: NSMenuItemValidation {
    /// The ticks and radio marks of the menus follow the live values.
    func validateMenuItem(_ item: NSMenuItem) -> Bool {
        switch item.action {
        case #selector(chooseTheme(_:)): item.state = (item.representedObject as? String) == themes.theme.id ? .on : .off
        case #selector(chooseAppearance(_:)): item.state = (item.representedObject as? String) == themes.mode.rawValue ? .on : .off
        case #selector(toggleStatusBar): item.state = look.statusBar ? .on : .off
        case #selector(toggleTranslucent): item.state = look.opacity < 1 ? .on : .off
        case #selector(toggleBlur): item.state = look.blur ? .on : .off
        case #selector(toggleLiquidGlass):
            item.state = look.liquidGlass ? .on : .off
            return WindowLook.macHasGlass
        case #selector(toggleDividers): item.state = look.columnDividers ? .on : .off
        default: break
        }
        return true
    }
}

extension AppController {
    /// The control folder's look commands; false when `command` is not one of them.
    func runLook(command: String, argument: String) -> Bool {
        func on(_ flag: inout Bool) { flag = argument == "on" ? true : argument == "off" ? false : !flag }
        switch command {
        case "theme":
            guard themes.library.theme(id: argument) != nil else { reply("error: no theme \(argument)"); return true }
            themes.setTheme(argument)
        case "appearance":
            guard let mode = ThemeMode(rawValue: argument) else { reply("error: appearance is light, dark or system"); return true }
            themes.mode = mode
        case "status-bar": on(&look.statusBar)
        case "blur": on(&look.blur)
        case "glass": on(&look.liquidGlass)
        case "dividers": on(&look.columnDividers)
        case "opacity":
            guard let value = Double(argument), (0.3...1).contains(value) else { reply("error: opacity is 0.3 to 1"); return true }
            look.opacity = value
        case "reduce-transparency", "increase-contrast":
            // The system's own switches are followed live; this stands in for them where a check cannot flip the setting.
            if command == "reduce-transparency" { on(&themes.reduceTransparency) } else { on(&themes.increaseContrast) }
        default: return false
        }
        reply("\(command) \(argument)")
        return true
    }

    /// One line of what the look is now, for `state`.
    var lookState: String {
        "theme=\(themes.theme.id) appearance=\(themes.appearance.rawValue) mode=\(themes.mode.rawValue) "
            + "opacity=\(look.opacity) blur=\(look.blur) glass=\(look.liquidGlass) status_bar=\(look.statusBar) "
            + "dividers=\(look.columnDividers) reduce_transparency=\(themes.reduceTransparency) increase_contrast=\(themes.increaseContrast)"
    }
}

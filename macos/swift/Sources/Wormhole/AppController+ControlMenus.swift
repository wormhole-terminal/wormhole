// The control folder's menu and popover commands (`WORMHOLE_DEVCTL`), GPUI's `devctl/menus.rs` and `title.rs`: `menu-native`, `popover`,
// `popover-press|type|key|colour|close|native|real|real-close`, `path-menu`, `files-menu` and `available`. The menus here are
// already native (`menu`, `menu-pick` read and press the one a right-click or a key opened, kept unshown); a popover is SwiftUI, so
// the one a script should reach describes itself while open (`DesignSystem.PopoverRecord`).

import AppKit
import DesignSystem
import Foundation
import PanelUI
import PlatformModel
import PlatformUI
import SwiftUI
import WindowUI
import WorkspaceModel

/// The popover `popover-real` opened, until `popover-real-close`.
@MainActor private enum RealPopover { static var shown: NSPopover? }

extension AppController {
    /// False when `command` is not one of these.
    func runMenus(command: String, argument: String) -> Bool {
        let argument = argument.trimmingCharacters(in: .whitespaces)
        switch command {
        case "menu-native": reply(menuNative(argument))
        case "path-menu":
            guard let menu = hub.selectedPathMenu else { reply("error: the selected terminal has no folder"); return true }
            injector.keep(menu)
            reply("path-menu\n" + Self.rows(of: menu))
        case "files-menu":
            hub.send(.showPanelTab(.files))
            guard let menu = FilesPathMenu.menu(for: hub.panel.files) else { reply("error: the tree has no folder yet"); return true }
            injector.keep(menu)
            reply("files-menu\n" + Self.rows(of: menu))
        case "available":
            // Whether the bar's item for the action is on for the window in front; an action with no item counts as available when
            // the action table runs it (GPUI's answer is the focused element's path, which has no counterpart here).
            let id = ActionID(rawValue: argument)
            reply("\(argument): \(menus?.availability(of: id) ?? (router.table[id] != nil))")
        case "popover": reply(PopoverRecord.listing)
        case "popover-press":
            guard let open = PopoverRecord.open else { reply("error: no popover is open"); return true }
            guard let press = open.rows.first(where: { $0.title == argument })?.press else { reply("error: no button or action \(argument)"); return true }
            press()
            reply("popover-press \(argument)")
        case "popover-type":
            guard let open = PopoverRecord.open else { reply("error: no popover is open"); return true }
            open.type(argument)
            reply("popover-type \(argument.debugDescription)")
        case "popover-key":
            guard let open = PopoverRecord.open else { reply("error: no popover is open"); return true }
            reply(open.key(argument) ? "popover-key \(argument)" : "error: popover-key enter|escape")
        case "popover-colour":
            guard let open = PopoverRecord.open else { reply("error: no popover is open"); return true }
            reply(open.colour(argument) ? "popover-colour \(argument)" : "error: no colour \(argument) (a name, none or #rrggbb)")
        case "popover-close":
            guard let open = PopoverRecord.open else { reply("error: no popover is open"); return true }
            open.close()
            reply("popover-close")
        case "popover-native": reply(popoverPicture(argument))
        case "popover-real", "popover-real-close": reply(realPopover(open: command == "popover-real"))
        default: return false
        }
        return true
    }

    /// `menu-native [press <n>]`: the kept menu as AppKit has it, each item with its key, marks and tooltip and a row of colour dots with
    /// their names; `press <n>` presses the n-th dot as a click on it does.
    private func menuNative(_ argument: String) -> String {
        guard let menu = injector.menu else { return "error: no menu is open" }
        let dots = menu.items.compactMap { item in SwatchMenuProbe.dots(of: item).map { (item, $0) } }
        if argument.hasPrefix("press") {
            guard let n = Int(argument.dropFirst(5).trimmingCharacters(in: .whitespaces)), n >= 1, let (item, names) = dots.first, names.indices.contains(n - 1)
            else { return "error: menu-native press <n> (a dot of the menu's row of colours)" }
            _ = injector.closeMenu()
            SwatchMenuProbe.press(dot: n - 1, of: item)
            return "menu-native press \(n): \(names[n - 1])"
        }
        var lines = [Self.rows(of: menu)]
        for item in menu.items where item.toolTip != nil { lines.append("tooltip of \(item.title.isEmpty ? "(item)" : item.title): \(item.toolTip ?? "")") }
        for (_, names) in dots { lines.append("dots: " + names.enumerated().map { "\($0.offset + 1). \($0.element)" }.joined(separator: " | ")) }
        return lines.joined(separator: "\n")
    }

    /// `popover-native [name]`: the open popover's content drawn to `<ctl>/<name>.png` as AppKit lays it out, in the theme's colours.
    private func popoverPicture(_ argument: String) -> String {
        guard let open = PopoverRecord.open, let folder = options.devctl else { return "error: no popover is open" }
        let name = argument.isEmpty ? "popover" : argument
        guard name.allSatisfy({ $0.isLetter || $0.isNumber || "._-".contains($0) }) else { return "error: a name takes letters, digits, . _ and - only" }
        let host = NSHostingView(rootView: open.view.themed())
        host.frame = NSRect(origin: .zero, size: host.fittingSize)
        host.layoutSubtreeIfNeeded()
        guard let bitmap = host.bitmapImageRepForCachingDisplay(in: host.bounds), let png = { () -> Data? in
            host.cacheDisplay(in: host.bounds, to: bitmap)
            return bitmap.representation(using: .png, properties: [:])
        }() else { return "error: no picture" }
        do {
            try png.write(to: folder.appendingPathComponent(name + ".png"))
            return "popover-native \(name).png \(Int(host.bounds.width))x\(Int(host.bounds.height))"
        } catch {
            return "error: popover-native failed: \(error)"
        }
    }

    /// `popover-real` / `popover-real-close`: the recorded popover opened for real (an `NSPopover` at the window's top left), and closed.
    private func realPopover(open: Bool) -> String {
        if !open {
            guard let popover = RealPopover.shown else { return "error: no real popover is open" }
            popover.performClose(nil)
            RealPopover.shown = nil
            return "popover-real-close"
        }
        guard let record = PopoverRecord.open, let content = window?.contentView else { return "error: no popover is open" }
        RealPopover.shown?.performClose(nil)
        let popover = NSPopover()
        popover.behavior = .applicationDefined
        popover.contentViewController = NSHostingController(rootView: record.view.themed())
        popover.show(relativeTo: NSRect(x: 0, y: content.bounds.height - 1, width: 1, height: 1), of: content, preferredEdge: .minY)
        RealPopover.shown = popover
        return "popover-real"
    }
}

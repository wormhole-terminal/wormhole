// What Settings says of the surfaces outside the window: the menu bar extra on or off, what the Dock badge counts and the
// global key (and what it does). Only a real instance has them at all (`--background` makes none, `AmbientController.init`).

import Foundation
import PlatformModel
import PlatformUI
import SettingsModel

extension AmbientController {
    func apply(_ values: [String: JSONValue], changed: (String...) -> Bool) {
        if changed("menu_bar") { status.item?.isVisible = values["menu_bar"]?.bool ?? true }
        if changed("menu_bar_icon") { iconMode = values["menu_bar_icon"]?.string.flatMap(MenuBarIconMode.init(rawValue:)) ?? .statusCount }
        if changed("menu_bar_animate") { animatesIcon = values["menu_bar_animate"]?.bool ?? false }
        // Show Wormhole's key beside its item in the status menu.
        if changed("hotkey") { pushStatus() }
        if changed("dock_badge") { badgeMode = values["dock_badge"]?.string.flatMap(DockBadgeMode.init(rawValue:)) ?? .needsYou }
    }
}

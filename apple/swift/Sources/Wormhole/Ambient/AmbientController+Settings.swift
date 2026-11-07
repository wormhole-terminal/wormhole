// What Settings says of the surfaces outside the window: the menu bar extra on or off, what the Dock badge counts and the
// global key (and what it does). Only a real instance has them at all (`--background` makes none, `AmbientController.init`).

import Foundation
import PlatformModel
import PlatformUI
import SettingsModel

extension AmbientController {
    func apply(_ values: [String: JSONValue], changed: (String...) -> Bool) {
        if changed("menu_bar") { status.item?.isVisible = values["menu_bar"]?.bool ?? true }
        if changed("dock_badge") { badgeMode = values["dock_badge"]?.string.flatMap(DockBadgeMode.init(rawValue:)) ?? .needsYou }
        if changed("hotkey", "hotkey_action") { setHotkey(values["hotkey"]?.string ?? "", action: values["hotkey_action"]?.string ?? "") }
    }

    private func setHotkey(_ text: String, action: String) {
        hotkey?.unregister()
        hotkey = nil
        guard !text.isEmpty, let spec = try? KeySpec(parsing: text) else { return }
        let key = GlobalHotkey { [weak self] in
            guard let self else { return }
            if action == "toggle_app" { toggleApp() } else { toggleQuickPanel() }
        }
        do {
            try key.register(spec)
            hotkey = key
        } catch {
            FileHandle.standardError.write(Data("hotkey \(text): \(error)\n".utf8))
        }
    }
}

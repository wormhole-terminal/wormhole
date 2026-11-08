// Zen mode (`settings::zen`): every window shown with the Zen preset for a while, nothing saved. The preset is the core's
// (`Zen.preset`); `WindowLook` layers it over the person's own values, so ending it brings back exactly what was there.
// The title row is the one part of it the windows hold themselves, so it is swapped here.

import AppKit
import WindowUI
import WordsModel
import WorkspaceModel

extension AppController {
    var zenOn: Bool { look.zen != nil }

    /// View ▸ Zen Mode, the panel's button while it is on, the settings note's button.
    func setZen(_ on: Bool) {
        guard on != zenOn, !on || Zen.preset != nil else { return }
        look.zen = on ? Zen.preset : nil
        settingsApplier?.applyZen()
        let words = on ? Words.text("zen-on", fallback: "Zen mode on") : Words.text("zen-off", fallback: "Zen mode off")
        NSAccessibility.post(element: NSApp as Any, notification: .announcementRequested, userInfo: [.announcement: words])
    }

    func toggleZen() { setZen(!zenOn) }
}

// text-file: a test driver's replies on the control folder, not interface text
// The control folder's `window-look`: what the window the command acts on is drawn with, each from the thing that draws it,
// so a window of another profile than the one Wormhole is in can be told from the others: the theme and the window colour of
// the theme store its views are given (`WindowSession.dress`), the status bar's switch of its own look, and the selected
// terminal's own point size and background (`MetalTerminalView`'s renderer). Replies:
//   window-look profile="<id>" theme="<name>" window="<#rrggbb, the theme's `surface.window`>" status_bar="<true|false>"
//       font="<pt>" terminal="<#rrggbb, the terminal's background>"       (`font` and `terminal` are `-` with no terminal shown)

import DesignSystem
import Foundation
import WindowUI

extension AppController {
    /// The look the window the command acts on is drawn with: its own profile's, which is the app's only for a window of the
    /// profile Wormhole is in. What a read-out of that window's bars is built from.
    var windowLook: WindowUI.WindowLook { sessions.first { $0.hub === hub }?.dress.look ?? look }

    /// False when `command` is not `window-look`.
    func runWindowLook(command: String) -> Bool {
        guard command == "window-look" else { return false }
        guard let session = sessions.first(where: { $0.hub === hub }) else { reply("error: no window"); return true }
        let look = windowLook
        let view = selectedView
        let terminal = view.map { view in
            let colour = view.renderer.palette.background
            return String(format: "#%02x%02x%02x", Int(colour.x), Int(colour.y), Int(colour.z))
        }
        reply(Readout.line(
            "window-look",
            pairs: [
                ("profile", session.windows.profile), ("theme", look.themes.theme.name), ("window", look.themes.tokens.rgba(named: "surface.window")?.hex ?? "-"),
                ("status_bar", String(look.statusBar)), ("font", view.map { String($0.pointSize) } ?? "-"), ("terminal", terminal ?? "-"),
            ]))
        return true
    }
}

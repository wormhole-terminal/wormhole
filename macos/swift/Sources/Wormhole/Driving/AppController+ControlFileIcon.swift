// text-file: a test driver's replies on the control folder, not interface text
// The control folder's `file-icon <name> [light]` and `file-icon dir:<name> [open] [light]`: the icon the rows draw for a
// file or a folder of that name, asked of the icon theme in force exactly as the Files tab, the Changes and the History rows
// ask it (`FileIconSource.shared`, which the app sets from the `icon_theme` setting). Replies:
//   file-icon "<name>" icon="<the SVG's path>" [colour]      `colour`: the icons keep their own colours
//   file-icon "<name>" icon="-"                              no icon: the row draws the system's symbol
// `file-icon themes`: the choices Settings → Appearance builds its Icon theme menu from (the host's list, which the menu is
// given), a `theme "<name>"` line each.

import DesignSystem
import FileIconTheme
import Foundation
import SettingsModel

extension AppController {
    /// False when `command` is not `file-icon`.
    func runFileIcon(command: String, argument: String) -> Bool {
        guard command == "file-icon" else { return false }
        var words = argument.split(separator: " ").map(String.init)
        guard !words.isEmpty else { reply("error: file-icon <name> [light] | dir:<name> [open] [light]"); return true }
        let name = words.removeFirst()
        if name == "themes" {
            reply((["file-icon themes"] + settingsModel.host.options(for: .iconThemes).map { Readout.line("theme", quoted: [$0.label]) }).joined(separator: "\n"))
            return true
        }
        let dark = !words.contains("light")
        let icons = FileIconSource.shared
        let icon = name.hasPrefix("dir:")
            ? icons.folder(String(name.dropFirst(4)), open: words.contains("open"), dark: dark) : icons.file(name, dark: dark)
        reply(Readout.line(
            "file-icon", quoted: [name], pairs: [("icon", icon?.path ?? "-")], flags: [Readout.flag("colour", icon != nil && icons.rendersInColour)]))
        return true
    }
}

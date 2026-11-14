// text-file: a test driver's replies on the control folder, not interface text
// The control folder's `usage` (the Usage tab as its store holds it, `PanelUI.UsageReadout`) and `recents [projects|folders]`:
// Open Recent's projects of the window's own profile (`RecentProjects`), or the folder picker's recent folders as that
// profile's database keeps them (`FolderPicking`). Replies:
//   recents projects profile="<id>" count="<n>"     then `project "<name>" path="<root>"` per root, most recent first
//   recents folders profile="<id>"                  then `json <the stored list>` (`folders: none` for a sample)

import DesignSystem
import PanelUI

extension AppController {
    /// False when `command` is not one of these.
    func runUsageRecents(command: String, argument: String) -> Bool {
        switch command {
        case "usage": reply(UsageReadout.text(hub.panel.usage))
        case "recents":
            // The window the command acts on: its own profile's lists, whichever profile Wormhole is in.
            let profile = sessions.first { $0.hub === hub }?.windows.profile ?? windowsHost.profile
            if argument.trimmingCharacters(in: .whitespaces) == "folders" {
                #if WITH_UNIFFI
                    reply(Readout.line("recents", ["folders"], pairs: [("profile", profile)]) + "\njson " + (folderPicking.recentFoldersJSON(of: profile) ?? "none"))
                #else
                    reply("recents folders: none")
                #endif
                return true
            }
            let roots = recents.roots(of: profile)
            reply(([Readout.line("recents", ["projects"], pairs: [("profile", profile), ("count", String(roots.count))])]
                + roots.map { Readout.line("project", quoted: [($0 as NSString).lastPathComponent], pairs: [("path", $0)]) }).joined(separator: "\n"))
        default: return false
        }
        return true
    }
}

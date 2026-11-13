// The control folder's profile commands (`WORMHOLE_DEVCTL`), GPUI's `devctl/sessions.rs`: `profiles` lists the core's profile list
// (the one Settings' Profile page reads, not the tab bar's chip, which is empty while there is one profile), `profile-open <id>`
// is a window in that profile, `profile-move <id>` sends the window in front there, `profile-delete <id>` is Settings' delete
// after its question. `profile-new` and `profile-switch` were already here (`AppController+ControlWindows`).

import Foundation
#if WITH_UNIFFI
    import SettingsBridge
    import WormholeBindings
#endif

extension AppController {
    /// False when `command` is not one of these.
    func runProfiles(command: String, argument: String) -> Bool {
        guard ["profiles", "profile-open", "profile-move", "profile-delete"].contains(command) else { return false }
        #if WITH_UNIFFI
            guard let host = settingsModel.host as? RustSettingsHost, let rows = try? host.profiles() else {
                reply("error: this instance has no profile list (it needs the real core)")
                return true
            }
            let id = argument.trimmingCharacters(in: .whitespaces)
            if command == "profiles" {
                // `* id "name" "colour"`: the active profile marked, as GPUI lists them.
                reply(rows.map { "\($0.isActive ? "*" : " ") \($0.id) \($0.name.debugDescription) \($0.color.debugDescription)" }.joined(separator: "\n"))
                return true
            }
            guard rows.contains(where: { $0.id == id }) else { reply("error: no profile \(id) (`profiles` lists them)"); return true }
            switch command {
            case "profile-open": openWindow(inProfile: id)
            case "profile-move": move(front, toProfile: id)
            default:
                // As Settings does it after its question: the profile's windows end first (`willDeleteProfile`), then the folder.
                if let problem = host.removeProfile(id) { reply("error: \(problem)"); return true }
            }
            reply("\(command) \(id)")
        #else
            reply("error: this build has no core")
        #endif
        return true
    }
}

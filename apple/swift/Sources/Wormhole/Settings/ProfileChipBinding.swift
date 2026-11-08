// The tab bar's profile chip over the core's profiles (`ProfileList` behind `profiles.json`, through the Settings host):
// who there is, switching, and the way to Settings' Profile page, where one is made and managed. A window of its own in a
// profile is not made yet: the menu leaves those entries out.

import PanesUI
import WorkspaceModel
#if WITH_UNIFFI
    import SettingsBridge
#endif

extension AppController {
    func profileChoices() -> [ProfileChoice] {
        #if WITH_UNIFFI
            guard let host = settingsModel.host as? RustSettingsHost, let rows = try? host.profiles() else { return [] }
            return rows.map { ProfileChoice(id: $0.id, name: $0.name, letter: $0.name.first ?? "?", current: $0.isActive, here: $0.isActive) }
        #else
            return []
        #endif
    }

    func profileActions() -> ProfileActions {
        ProfileActions(
            switchTo: { [weak self] id in
                #if WITH_UNIFFI
                    try? (self?.settingsModel.host as? RustSettingsHost)?.switchProfile(to: id)
                #endif
            },
            newProfile: { [weak self] in self?.showSettings(); self?.settingsController?.show(section: "profile") },
            manage: { [weak self] in self?.showSettings(); self?.settingsController?.show(section: "profile") },
            settings: { [weak self] in self?.showSettings() })
    }
}

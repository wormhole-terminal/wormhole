// The app's own actions: its secondary windows, quitting and where it keeps its files.

import AppKit
import PlatformModel

extension ActionRouter {
    func appActions() -> [ActionID: Handler] {
        [
            "newWindow": { [unowned self] in app.openNewWindow() },
            "newProfile": { [unowned self] in app.showProfiles(new: true) },
            "manageProfiles": { [unowned self] in app.showProfiles() },
            "settings": { [unowned self] in app.showSettings() },
            "about": { [unowned self] in app.aux.showAbout() },
            "checkForUpdates": { [unowned self] in app.aux.showUpdates() },
            "licenses": { [unowned self] in app.aux.showLicenses() },
            "sessionHosts": { [unowned self] in app.aux.showHosts() },
            "showIntro": { [unowned self] in app.aux.showWelcome() },
            "themeStudio": { [unowned self] in app.studio.show() },
            "openFolder": { [unowned self] in app.openFolder() },
            "showLogs": { [unowned self] in app.showLogs() },
            "quitAndEndAll": { [unowned self] in app.quit(endAll: true) },
        ]
    }
}

// What the system asks of the running app from outside: a file or folder to open (a double-clicked `.command`, Open With, a
// folder on the Dock icon, `open -a Wormhole <folder>`: the document types in Info.plist), and a second launch's Activate. What a
// terminal does with a file is core's (`terminal::default_terminal`, through `HookServer.openFromSystem`). SIGTERM is
// `AppController+WindowEnding.swift`'s.

import AppKit

extension AppController {
    /// Asked to open these: kept until the windows and the socket are there (a launch to open a file asks before they are).
    func application(_ application: NSApplication, open urls: [URL]) {
        systemOpens += urls
        openWhatSystemAsked()
    }

    /// A terminal for each waiting URL in the window in front, which comes forward.
    func openWhatSystemAsked() {
        guard let hooks, !systemOpens.isEmpty, !sessions.isEmpty else { return }
        let urls = systemOpens
        systemOpens = []
        let session = front
        hooks.openFromSystem(urls, in: session.id)
        if policy.touchesSystem {
            session.window?.makeKeyAndOrderFront(nil)
            NSApp.activate()
        }
    }

    /// A second launch of this data folder asked the running one to come forward: with no window open, one opens.
    func activateFromSecondLaunch() {
        if !shown.isEmpty && shown.allSatisfy({ $0.window.map { !$0.isVisible && !$0.isMiniaturized } ?? true }) { openNewWindow() }
        if policy.touchesSystem { NSApp.activate() }
    }
}

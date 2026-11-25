// A launch that cannot go on says why to a person who started the app from its icon: there is no terminal to read a stderr line in.
// An instance a script started (`--background`, `WORMHOLE_BACKGROUND=1`, or beside a control folder) never puts a question on the
// screen, which would bring a window forward and hang whatever waits for it, and neither does a command line (it exits before the
// app is set up) or a bare executable (a developer's run, whose terminal shows the line).

import AppKit
import Foundation
import WordsModel
#if WITH_UNIFFI
    import WordsBridge
#endif

enum LaunchFailure {
    /// Whether the failure is shown: only to an app launched as one (`options` nil is a launch that did not get as far as its own options,
    /// a mistake in the command line, which says so on stderr as it always did).
    static func showsAlert(options: Options?, isAppBundle: Bool = Bundle.main.bundleURL.pathExtension == "app") -> Bool {
        guard let options, isAppBundle else { return false }
        return !options.background && options.devctl == nil
    }

    /// The failure said on screen when the launch is one that shows it; the caller exits after it, as before.
    @MainActor
    static func show(_ error: Error, options: Options?, dataDirectory: URL?) {
        guard showsAlert(options: options) else { return }
        #if WITH_UNIFFI
            // The words may not be installed yet: a launch that fails before the app is set up still speaks the person's language.
            WordsBridge.install()
        #endif
        let info = Bundle.main.infoDictionary
        let name = info?["CFBundleDisplayName"] as? String ?? info?["CFBundleName"] as? String ?? "Wormhole" // text: the product's name
        var detail = "\(error)"
        if let dataDirectory {
            let logs = dataDirectory.appendingPathComponent("logs").path
            detail += "\n\n" + Words.text("launch-failed-logs", ["folder": .text(logs)], fallback: "The logs are in \(logs).")
        }
        let alert = NSAlert()
        alert.alertStyle = .critical
        alert.messageText = Words.text("launch-failed-title", ["name": .text(name)], fallback: "\(name) can't start")
        alert.informativeText = detail
        alert.addButton(withTitle: Words.text("launch-failed-quit", fallback: "Quit"))
        // The run loop has not started (or is just about to), and the app may not have a window: the question is its own.
        NSApplication.shared.setActivationPolicy(.regular)
        NSApplication.shared.activate()
        alert.runModal()
    }
}

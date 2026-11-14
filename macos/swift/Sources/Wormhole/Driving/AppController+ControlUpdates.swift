// text-file: a test driver's replies on the control folder, not interface text
// The control folder's `update-status`: Settings → Updates as the open Settings window draws it, from the model its page is
// given (`SettingsWindowController.updates`, whose `statusLine` is the page's own text) and the buttons the page laid out
// (`RowFrames`: `update.check`, `update.skip`, `update.install`, `update.restart`, `update.keep`; `settings-press` clicks one).
// Replies:
//   update-status <idle|checking|up-to-date|available|unsupported|downloading|installed|failed> "<the status line>"
//       channel="<stable|preview>" [version="<the release offered or installed>"] [note="<what a channel change did>"] [pending] [security]
//   button <id>              per button on the page, left to right
//   notes "<line>"           per line under What's new, while a release is offered
//   update-status none       a build without an updater (the page says so itself: `settings-page`)
//   error: …                 Settings is not open

import AuxModel
import DesignSystem
import SettingsUI

extension AppController {
    /// False when `command` is not `update-status`.
    func runUpdates(command: String) -> Bool {
        guard command == "update-status" else { return false }
        guard let settings = settingsController, settings.window?.isVisible == true else {
            reply("error: Settings is not open (`updates` opens it on Updates)")
            return true
        }
        guard let model = settings.updates else { reply("update-status none"); return true }
        // The channel is the updater's own (Settings' row writes the setting, which the app hands to the updater), not the
        // page model's copy, which nothing draws.
        var pairs = [("channel", model.backend.channel == .stable ? "stable" : "preview")]
        var notes: [String] = []
        var security = false
        let word: String
        switch model.phase {
        case .idle: word = "idle"
        case .checking: word = "checking"
        case .upToDate: word = "up-to-date"
        case .available(let release):
            word = "available"
            pairs.append(("version", release.version))
            notes = UpdateViewModel.noteLines(release.notes)
            security = release.isSecurity
        case .unsupported: word = "unsupported"
        case .downloading: word = "downloading"
        case .installed(let version):
            word = "installed"
            pairs.append(("version", version))
        case .failed: word = "failed"
        }
        if let note = model.channelNote { pairs.append(("note", note)) }
        let head = Readout.line(
            "update-status", [word], quoted: [model.statusLine], pairs: pairs,
            flags: [Readout.flag("pending", model.restartPending), Readout.flag("security", security)])
        let buttons = RowFrames.all.filter { $0.id.hasPrefix("update.") }.sorted { $0.frame.minX < $1.frame.minX }.map { Readout.line("button", [$0.id]) }
        reply(([head] + buttons + notes.map { Readout.line("notes", quoted: [$0]) }).joined(separator: "\n"))
        return true
    }
}

// The control folder's commands for what Settings holds and what hangs off it: `set key=<json>` writes a setting as a
// control of the window would (so the window follows), `get key` reads one back, `settings-section <id>` opens a page,
// `studio` and `folders` open the Theme Studio and the folder picker, `fonts` says what the terminals are drawn with.

import Foundation
import SettingsModel

extension AppController {
    /// False when `command` is not one of these.
    func runSettings(command: String, argument: String) -> Bool {
        switch command {
        case "set":
            let parts = argument.split(separator: "=", maxSplits: 1).map(String.init)
            guard parts.count == 2, let data = parts[1].data(using: .utf8),
                let value = try? JSONDecoder().decode(JSONValue.self, from: data)
            else { reply("error: usage set key=<json value>"); return true }
            settingsModel.set(parts[0], value)
            reply("set \(parts[0]) -> \(settingsModel.value(parts[0]))\(settingsModel.backend.refusal.map { " refused: \($0)" } ?? "")")
        case "get": reply("\(argument) = \(settingsModel.value(argument))")
        case "settings-section":
            showSettings()
            settingsController?.show(section: argument)
            reply("settings-section \(argument)")
        case "studio": studio.show(); reply("studio")
        case "folders": folderPicking.open(); reply("folders")
        case "fonts":
            let fonts = terminalLook.fonts
            reply("fonts=\(fonts.map { "\($0.font) size=\($0.pointSize) line=\($0.lineHeight ?? 0) ligatures=\($0.ligatures)" } ?? "none") view=\(selectedView.map { "\($0.pointSize)pt" } ?? "-")")
        default: return false
        }
        return true
    }
}

extension SettingsBackend {
    /// Core's sentence for the last write it refused, when the backend says (the sample never does).
    var refusal: String? { (self as? any RefusalReporting)?.lastRefusal }
}

/// A backend that keeps the sentence of the last refused write.
@MainActor
protocol RefusalReporting { var lastRefusal: String? { get } }

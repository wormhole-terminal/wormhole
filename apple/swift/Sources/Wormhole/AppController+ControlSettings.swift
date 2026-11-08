// The control folder's commands for what Settings holds and what hangs off it: `set key=<json>` writes a setting as a
// control of the window would (so the window follows), `get key` reads one back, `settings-section <id>` opens a page, `flags-unlock` does what ⌃⌥⌘F does, `settings-size WxH` resizes its window, `profile k=v …` edits the profile as the fields do, `import-open|import-apply <id>`,
// `studio` and `folders` open the Theme Studio and the folder picker, `fonts` says what the terminals are drawn with.

import AppKit
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
        case "flags-unlock":
            // ⌃⌥⌘F in the window: unlocks the feature flags (and opens their page) or locks them again.
            let unlocked = settingsModel.host.toggleFlagsUnlocked()
            if unlocked { settingsModel.show("featureFlags") }
            reply("flags-unlock unlocked=\(unlocked) nav=\(settingsModel.navSections.map(\.id).joined(separator: ","))")
        case "settings-size":
            // A taller window shows more of a page in one `window-shot`.
            let size = argument.split(separator: "x").compactMap { Double($0) }
            guard size.count == 2 else { reply("error: settings-size WxH"); return true }
            showSettings()
            settingsController?.window?.setContentSize(NSSize(width: size[0], height: size[1]))
            reply("settings-size \(argument)")
        case "settings-search":
            showSettings()
            settingsModel.query = argument
            reply("settings-search \(argument) hits=\(settingsModel.hits.count) sections=\(settingsModel.hitCounts.count)")
        case "profile":
            // `profile name=Home color=green env=A=b`: the fields a person types, committed as blur does.
            let draft = settingsModel.profile
            for pair in argument.split(separator: " ", maxSplits: 8).map(String.init) {
                let parts = pair.split(separator: "=", maxSplits: 1).map(String.init)
                guard parts.count == 2 else { continue }
                switch parts[0] {
                case "name": draft.form?.name = parts[1]
                case "avatar": draft.form?.avatar = parts[1]
                case "color": draft.form?.color = parts[1]
                case "start": draft.form?.startDir = parts[1]
                case "env": draft.form?.env = parts[1].replacingOccurrences(of: ";", with: "\n")
                case "git": draft.form?.gitName = parts[1]
                case "folder": draft.form?.folders.append(parts[1])
                default: break
                }
            }
            draft.commit()
            reply("profile \(draft.form.map { "\($0.name)|\($0.letter)|\($0.color)|\($0.startDir)|\($0.folders)|\($0.env)" } ?? "none") problem=\(draft.problem ?? "none")")
        case "import-open":
            settingsModel.importDraft.toggle(argument)
            reply("import-open open=\(settingsModel.importDraft.open ?? "none") reviews=\(settingsModel.importDraft.reviews.keys.sorted())")
        case "import-apply":
            settingsModel.importDraft.apply(argument)
            reply("import-apply \(settingsModel.importDraft.outcome?.text ?? "nothing")")
        case "probe-approve":
            // The Extensions page's approval with every permission left ticked, through the sheet's own model: the owner's question
            // is answered by WORMHOLE_AUTH in this instance, never by Touch ID.
            guard let probes = settingsModel.probesModel else {
                reply("error: no Probes backend")
                return true
            }
            Task { @MainActor in
                await probes.openSheet(argument)
                for permission in probes.open?.sheet.permissions ?? [] where probes.open?.granted.contains(permission.id) != true {
                    await probes.toggle(permission)
                }
                await probes.approve()
                reply("probe-approve \(argument) \(probes.list?.rows.first { $0.id == argument }?.state.rawValue ?? "?") \(probes.open?.error ?? "")")
            }
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

// The control folder's commands for what Settings holds and what hangs off it: `set key=<json>` writes a setting as a
// control of the window would (so the window follows), `get key` reads one back, `settings-section <id>` opens a page, `flags-unlock` does what ⌃⌥⌘F does, `settings-size WxH` resizes its window, `profile k=v …` edits the profile as the fields do, `import-open|import-apply <id>`,
// `studio` and `folders` open the Theme Studio and the folder picker, `fonts` says what the terminals are drawn with.

import AppKit
import CoreText
import DesignSystem
import Foundation
import SettingsModel
import SettingsUI
import TerminalMetal

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
        case "flags":
            // The feature flags that are on as the window in front has them (its own profile's, read from the file a moment
            // after `set` wrote it): what a driver waits on before it relies on a flag.
            reply("flags on=" + flagsOn(front).sorted().joined(separator: ","))
        case "panel-pick":
            // The answer of the next open panel of Settings (Choose app…, a folder, Import theme…); a test instance shows none.
            PanelPick.answer(with: argument.isEmpty ? nil : URL(fileURLWithPath: argument))
            reply("panel-pick \(argument.isEmpty ? "(cancel)" : argument)")
        case "reset":
            // What a row's reset arrow does: the keys back at their defaults.
            settingsModel.backend.reset(argument.split(separator: " ").map(String.init))
            reply("reset \(argument) -> " + argument.split(separator: " ").map { "\($0) = \(settingsModel.value(String($0)))" }.joined(separator: ", "))
        case "look-keys": reply(settingsEffects)
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
        case "settings-rows":
            // The page on show, one row a line: `id | keys | kind | value | x y w h` of its control (in the window's points; the
            // reset arrow as `id.reset`), so a driver clicks the control itself.
            let entries = settingsModel.schema.sections.flatMap { $0.groups.flatMap(\.entries) }
            let frames = Dictionary(RowFrames.all.map { ($0.id, $0.frame) }, uniquingKeysWith: { $1 })
            for (id, frame) in RowFrames.all {
                let entry = entries.first { $0.id == id.replacingOccurrences(of: ".reset", with: "") }
                let value = entry.map { $0.keys.map { "\($0)=\(settingsModel.value($0))" }.joined(separator: ",") } ?? ""
                let kind = entry.map { String(describing: $0.control).prefix(14) } ?? "-"
                reply("\(id) | \(value) | \(kind) | \(Int(frame.minX)) \(Int(frame.minY)) \(Int(frame.width)) \(Int(frame.height))")
            }
            if frames.isEmpty { reply("settings-rows: none on show") }
        case "settings-control":
            // The segments of a segmented control (the centre of each, in the window's points) or the views inside a row's frame.
            guard let window = settingsController?.window, let content = window.contentView, let row = RowFrames.all.first(where: { $0.id == argument })
            else { reply("error: settings-control <row id> (a row on show)"); return true }
            let point = injector.location(Double(row.frame.midX), Double(row.frame.midY), in: window)
            let chain = sequence(first: injector.view(at: point, in: window), next: { $0?.superview }).compactMap { $0 }
            if let seg = chain.lazy.compactMap({ $0 as? NSSegmentedControl }).first {
                // Each segment's own accessibility frame (the widths are automatic).
                let ranges = (seg.accessibilityChildren() ?? []).compactMap { child -> String? in
                    guard let node = child as? NSAccessibilityProtocol else { return nil }
                    let local = content.convert(window.convertFromScreen(node.accessibilityFrame()), from: nil)
                    return "\(node.accessibilityLabel() ?? "?")@\(Int(local.midX)),\(Int(local.midY))"
                }
                reply("segments \(argument): " + ranges.joined(separator: " "))
            } else {
                reply("control \(argument): " + chain.prefix(3).map { "\(type(of: $0))" }.joined(separator: " < "))
            }
        case "settings-pick":
            // A pop-up's pick by the item's words: the system's menu picker is drawn by SwiftUI and takes no injected click, so
            // this does what its binding does with the chosen item (`model.set` of the option's value).
            let words = argument.split(separator: " ", maxSplits: 1).map(String.init)
            let entry = settingsModel.schema.sections.flatMap { $0.groups.flatMap(\.entries) }.first { $0.id == words.first }
            guard words.count == 2, let entry, let key = entry.keys.first else { reply("error: settings-pick <row id> <item words>"); return true }
            let options: [SettingOption]
            switch entry.control {
            case .select(let list, _): options = list
            case .segmented(let list, _, _): options = list
            case .dynamicSelect(let source): options = settingsModel.host.options(for: source)
            default: reply("error: \(entry.id) has no pop-up"); return true
            }
            guard let option = options.first(where: { $0.label == words[1] }) else {
                reply("error: no item \(words[1]) among \(options.map(\.label))")
                return true
            }
            settingsModel.set(key, option.value)
            reply("settings-pick \(entry.id) \(words[1]) -> \(key) = \(settingsModel.value(key))")
        case "settings-search":
            showSettings()
            settingsModel.query = argument
            reply("settings-search \(argument) hits=\(settingsModel.hits.count) sections=\(settingsModel.hitCounts.count)")
            // The row ids found, best first, and how many each page holds (the nav's counts).
            reply("  ids " + settingsModel.hits.map { "\($0.section)/\($0.entry.id)" }.joined(separator: " "))
            reply("  pages " + settingsModel.hitCounts.sorted { $0.key < $1.key }.map { "\($0.key)=\($0.value)" }.joined(separator: " "))
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
            // The card's rows are read a moment after it opens; Apply is on the card only once they are there.
            guard settingsModel.importDraft.reviews[argument] != nil else {
                reply("error: \(argument)'s settings are not read yet (`import-open \(argument)` opens its card; ask again)")
                return true
            }
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
            // The family CoreText gives the regular face now: another than the one set means a fallback (a custom font not registered).
            var resolved = ""
            if case .system(let family)? = fonts?.font {
                resolved = " resolved=" + (CTFontCopyFamilyName(CTFontCreateWithName(family as CFString, 12, nil)) as String)
            }
            reply("fonts=\(fonts.map { "\($0.font) size=\($0.pointSize) line=\($0.lineHeight ?? 0) ligatures=\($0.ligatures)" } ?? "none") view=\(selectedView.map { "\($0.pointSize)pt" } ?? "-")\(resolved)")
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

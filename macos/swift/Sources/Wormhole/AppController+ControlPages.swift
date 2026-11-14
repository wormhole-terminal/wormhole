// The control folder's commands for the secondary windows and the Settings window's own buttons (`WORMHOLE_DEVCTL`), GPUI's
// `devctl/menus.rs`, `settings.rs` and `welcome.rs`: `hosts-toggle`, `hosts-scroll`, `licenses-search`, `licenses-select`,
// `intro-frames`, `onboarding-tailor`, `welcome-probe`, `update-installed`, `import-scan`, `font-picker`, `settings-record`,
// `settings-reveal`, `settings-reset-confirm`, `settings-cli-tool`. Each does what the window's own control does, through its
// model. `settings-preview` has no counterpart: the Swift Terminals scene shows the three densities side by side, with no
// hover preview of one.

import AppKit
import AuxModel
import AuxWindows
import DesignSystem
import Foundation
import SettingsModel
import SettingsUI
import WordsModel
import WorkspaceModel
#if WITH_UNIFFI
    import AuxBridge
    import HookBridge
    import SettingsBridge
#endif

extension AppController {
    /// False when `command` is not one of these.
    func runPages(command: String, argument: String) async -> Bool {
        let argument = argument.trimmingCharacters(in: .whitespaces)
        switch command {
        case "hosts-toggle":
            guard let model = HostsWindow.model else { reply("error: the Hosts window is not open (`hosts`)"); return true }
            guard let id = argument.isEmpty ? model.report?.cards.first?.id : argument else { reply("error: no host to toggle"); return true }
            model.toggle(id)
            reply("hosts-toggle \(id) open=\(model.open.contains(id))")
        case "hosts-scroll":
            guard let model = HostsWindow.model, let offset = Double(argument) else { reply("error: hosts-scroll <points> (with the Hosts window open)"); return true }
            model.scroll(to: offset)
            reply("hosts-scroll \(argument)")
        case "licenses-search":
            guard let browser = Licenses.browser else { reply("error: the Licenses window is not open (`licenses`)"); return true }
            browser.query = argument
            reply("licenses-search \(argument.debugDescription) \(browser.countLine) selected=\(browser.selectedPackage?.name ?? "none")")
        case "licenses-select":
            guard let browser = Licenses.browser else { reply("error: the Licenses window is not open (`licenses`)"); return true }
            guard let index = browser.list.packages.firstIndex(where: { $0.name == argument }) else { reply("error: no package \(argument)"); return true }
            // A package the search hides is shown: the query is cleared, as choosing it by name implies.
            if !browser.matches.contains(index) { browser.query = "" }
            browser.selected = index
            reply("licenses-select \(argument) \(browser.selectedPackage?.version ?? "")")
        case "intro-frames": reply(introFrames(argument))
        case "onboarding-tailor":
            guard let look = aux.onboarding?.session.look else { reply("error: the intro window is not open (`onboarding look`)"); return true }
            let acts: [String: TailorAct] = ["back": .back, "over": .startOver, "ok": .settle]
            guard let act = acts[argument] ?? Int(argument).map({ TailorAct.answer($0 - 1) }) else { reply("error: onboarding-tailor <n|back|over|ok>"); return true }
            look.act(act)
            reply("onboarding-tailor \(argument) -> " + (look.walk.map { "\($0.stage) \($0.counter) \($0.question)" } ?? "no walk"))
        case "welcome-probe": reply(welcomeProbe(argument))
        case "update-installed": reply(updateInstalled(argument))
        case "import-scan": await importScan(argument)
        case "font-picker":
            let words = argument.split(separator: " ", maxSplits: 1).map(String.init)
            guard let slot = words.first, let picker = SettingsFontPickers.model(for: slot) else {
                reply("error: font-picker <interface|code|terminal> [query] (with the Fonts page on show: `settings-section fonts`)")
                return true
            }
            picker.isOpen = true
            picker.query = words.count > 1 ? words[1] : ""
            reply("font-picker \(slot) entries=\(picker.entries.count) highlighted=\(picker.highlighted ?? "-") preview=\(picker.previewFamily)")
        case "settings-record": await settingsRecord(argument)
        case "settings-reveal":
            showSettings()
            settingsModel.reveal(argument)
            reply(settingsModel.revealed == argument ? "settings-reveal \(argument) section=\(settingsModel.selection)" : "error: no setting \(argument)")
        case "settings-reset-confirm":
            showSettings()
            let count = settingsModel.changedKeyCount
            guard count > 0 else { reply("settings-reset-confirm: every setting is at its default, the button is off"); return true }
            // The question is held for `question confirm|cancel` in a test instance; nothing is reset until it is confirmed.
            SettingsActions.askReset(settingsModel, label: Words.text("settings-advanced-reset-all", fallback: "Reset all settings"), count: count)
            reply("settings-reset-confirm \(count) settings would go back (`question` says the question)")
        case "settings-cli-tool": reply(settingsCommandLine(argument))
        default: return false
        }
        return true
    }

    /// `intro-frames <style> [count]`: frames of the intro drawn off screen to `<ctl>/intro-<style>-<n>.bmp` (GPUI writes the same).
    private func introFrames(_ argument: String) -> String {
        let words = argument.split(separator: " ").map(String.init)
        guard let style = words.first.flatMap({ IntroStyle(rawValue: $0) }), let folder = options.devctl else {
            return "error: intro-frames <\(IntroStyle.allCases.map(\.rawValue).joined(separator: "|"))> [count]"
        }
        let count = min(max(words.count > 1 ? Int(words[1]) ?? 8 : 8, 1), 60)
        do {
            let written = try IntroFrames.write(style, count: count, colors: IntroColors(tokens: ThemeStore.shared.tokens), to: folder)
            return "intro-frames \(style.rawValue) \(written.count) frames: " + written.map(\.lastPathComponent).joined(separator: " ")
        } catch {
            return "error: intro-frames failed: \(error)"
        }
    }

    /// `welcome-probe <tab|-> <token> <op as JSON>`: a welcome request as the program sends it, from that tab (`-`: from outside any);
    /// the reply's JSON line.
    private func welcomeProbe(_ argument: String) -> String {
        #if WITH_UNIFFI
            let words = argument.split(separator: " ", maxSplits: 2).map(String.init)
            guard words.count == 3 else { return "error: welcome-probe <tab|-> <token> <op as JSON>" }
            guard let handle = hooks?.handle else { return "error: this instance serves no control socket" }
            return handle.welcomeLine(from: words[0] == "-" ? nil : words[0], token: words[1], op: words[2])
        #else
            return "error: this build has no control socket"
        #endif
    }

    /// `update-installed <version> [pending]`: Settings → Updates as after an install: Restart, or with `pending` the line that says
    /// it restarts when the terminals are idle.
    private func updateInstalled(_ argument: String) -> String {
        #if WITH_UNIFFI
            let words = argument.split(separator: " ").map(String.init)
            guard let version = words.first, let backend = aux.updateBackend() as? FFIUpdateBackend else { return "error: update-installed <version> [pending] (needs the real updater)" }
            backend.pretend(installed: version, pending: words.dropFirst().first == "pending")
            return "update-installed \(argument)"
        #else
            return "error: this build has no updater"
        #endif
    }

    /// `import-scan [<fixture home>]`: Settings → Import over a folder standing in for the home (`~/…` is the instance's), shown and
    /// open; with none, the instance's own home. Waits for the scan, then says what it found.
    private func importScan(_ argument: String) async {
        #if WITH_UNIFFI
            guard let host = settingsModel.host as? RustSettingsHost else { return reply("error: import-scan needs the real core") }
            host.importHome = argument.isEmpty ? nil : HomeFolder.expanding(argument)
            showSettings()
            settingsModel.show("import")
            settingsModel.importDraft.reset()
            host.perform(.scanImports)
            for _ in 0..<100 where host.panelRecords().scanningImports { await pause(50) }
            let found = host.panelRecords().imports
            reply("import-scan \(host.importHome ?? "home"): " + (found.isEmpty ? "none found" : found.map { "\($0.id) (\($0.app): \($0.summary))" }.joined(separator: ", ")))
        #else
            reply("error: import-scan needs the real core")
        #endif
    }

    /// `settings-record <id> [keys]`: a shortcut row's recorder started as a click on it starts it, then each key pressed into it.
    private func settingsRecord(_ argument: String) async {
        let words = argument.split(separator: " ").map(String.init)
        guard let id = words.first, let row = RowFrames.all.first(where: { $0.id == id }), let window = settingsController?.window else {
            return reply("error: settings-record <row id> [keys] (a shortcut row on show: `settings-rows`)")
        }
        let point = injector.location(Double(row.frame.midX), Double(row.frame.midY), in: window)
        var said = [injector.press(point, in: window, clicks: 1, flags: [])]
        if injector.isPressed { said.append(injector.release(point, in: window, clicks: 1, flags: [])) }
        // The recorder takes the keys a turn or two after the click (its capture view is made, then made first responder): a
        // key sent before that would go to the window and run a menu item. Waited for, and said when it never comes.
        for _ in 0..<40 where !(window.firstResponder is any RecordsKeys) { await pause(50) }
        guard window.firstResponder is any RecordsKeys else {
            return reply("error: the click on \(id) did not start its recorder: " + said.joined(separator: ", "))
        }
        // The keys go to the Settings window, as `pointer-window Settings` + `key` would send them.
        let before = injector.windowTitle
        injector.windowTitle = window.title
        for key in words.dropFirst() { said.append(sendKey(key)) }
        injector.windowTitle = before
        reply("settings-record \(argument): " + said.joined(separator: ", "))
    }

    /// `settings-cli-tool [install|uninstall]`: Coding agents → Command line tool: what it says, or what its Install / Uninstall does
    /// (into the instance's own `WORMHOLE_CLI_DIR`).
    private func settingsCommandLine(_ argument: String) -> String {
        showSettings()
        switch argument {
        case "install": settingsModel.host.perform(.installCommandLine)
        case "uninstall": settingsModel.host.perform(.uninstallCommandLine)
        case "": break
        default: return "error: settings-cli-tool [install|uninstall]"
        }
        let records = settingsModel.host.panelRecords()
        guard let tool = records.commandLine else { return "settings-cli-tool: not built in this instance" }
        return "settings-cli-tool \(tool.state) current=\(tool.current) folder=\(tool.folder ?? "-")\nheadline: \(tool.headline)\nnote: \(tool.note ?? "-")\npath: \(records.commandLinePath?.text ?? "-")"
    }
}

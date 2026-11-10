// The control folder's commands for what a button on a page or window does, for a driver with no pointer: the Updates
// window's check and install, the Extensions page's switch and sheet, the Project folders page's trust and choices, and
// the Theme Studio's draft. Each calls the page's own model, so the core answers as it does for a click.

import AuxModel
import Foundation
import SettingsModel
import StudioModel

extension AppController {
    func runVerifyPages(command: String, argument: String) -> Bool {
        switch command {
        case "update": runUpdate(argument)
        case "probe": return runProbe(argument)
        case "project": return runProject(argument)
        case "studio-do": runStudio(argument)
        case "studio-out": runStudioOutputs(argument)
        case "onboarding-step": aux.showOnboarding(step: argument); reply("onboarding-step \(argument)")
        case "intro", "welcome": runIntro(command: command, argument: argument)
        default: return false
        }
        return true
    }

    /// `update check`, `update install`; `update` alone says the phase.
    private func runUpdate(_ argument: String) {
        let backend = aux.updateBackend()
        Task { @MainActor in
            switch argument {
            case "check": await backend.check()
            case "install": if case .available(let release) = backend.phase { await backend.install(release) }
            default: break
            }
            self.reply("update \(argument) phase=\(backend.phase) channel=\(backend.channel)")
        }
    }

    /// `probe list`, `probe sheet|approve|off <id>`.
    private func runProbe(_ argument: String) -> Bool {
        guard let probes = settingsModel.probesModel else { reply("error: the probes flag is off"); return true }
        let words = argument.split(separator: " ").map(String.init)
        Task { @MainActor in
            await probes.refresh()
            if words.count > 1 {
                switch words[0] {
                case "sheet": await probes.openSheet(words[1])
                case "approve": await probes.openSheet(words[1]); await probes.approve()
                case "off": await probes.turnOff(words[1])
                default: break
                }
            }
            self.reply("probe \(argument): rows=\(probes.list?.rows.map { "\($0.id):\($0.state)" } ?? []) sheet=\(probes.open.map { "\($0.sheet)" } ?? "none") notice=\(probes.notice ?? "-")")
        }
        return true
    }

    /// `project list`, `project select <root>|all`, `project default <key> on|off`, `project choice <feature> on|off|default`,
    /// `project trust`, `project revoke <root>`.
    private func runProject(_ argument: String) -> Bool {
        guard let model = settingsModel.projectsModel else { reply("error: the project folders flag is off"); return true }
        let words = argument.split(separator: " ").map(String.init)
        Task { @MainActor in
            await model.refresh()
            switch (words.first, words.count) {
            case ("select", 2): await model.select(words[1] == "all" ? nil : words[1])
            case ("default", 3): await model.setDefault(words[1], on: words[2] == "on")
            case ("choice", 3): await model.setChoice(words[1], to: ["on": true, "off": false][words[2]])
            case ("trust", _): await model.trust()
            case ("revoke", 2): await model.revoke(words[1])
            default: break
            }
            self.reply("project \(argument):\n  defaults=\(model.defaults)\n  trusted=\(model.trusted)\n  known=\(model.known)\n  page=\(model.page.map { "\($0)" } ?? "none")")
        }
        return true
    }

    /// `studio-do state|area <name>|select <token>|set <token> <value>|reset <token>|rename <name>|undo|redo|save|revert|versions` on the open Studio's draft.
    private func runStudio(_ argument: String) {
        guard let document = studio.document else { return reply("error: the Studio is not open (`studio`)") }
        let words = argument.split(separator: " ", maxSplits: 2).map(String.init)
        var said = ""
        switch words.first {
        case "set": said = " ok=\(words.count == 3 && document.set(words[1], to: words[2]))"
        case "reset": said = " ok=\(words.count > 1 && document.reset(words[1]))"
        case "rename": document.rename(words.dropFirst().joined(separator: " "))
        case "area": document.filter.area = words.count > 1 ? StudioArea(rawValue: words[1]) ?? .all : .all
        case "select": document.selected = words.count > 1 ? words[1] : nil
        case "undo": document.undo()
        case "redo": document.redo()
        case "revert": document.revertToSaved()
        case "save": said = (try? document.save()).map { " saved=\($0.path)" } ?? " save failed"
        case "versions": said = " versions=\(document.versionRows().count)"
        default: break
        }
        reply("studio-do \(argument)\(said): name=\(document.name) dirty=\(document.isDirty) changes=\(document.changeCount) counts=\(document.counts) issues=\(document.contrastIssues.count) undo=\(document.undoLabel ?? "-") redo=\(document.redoLabel ?? "-")")
    }
}

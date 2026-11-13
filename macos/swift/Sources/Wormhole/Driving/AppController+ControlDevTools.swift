// text-file: a test driver's replies on the control folder, not interface text
// The rest of GPUI devctl's tooling on the control folder (`WORMHOLE_DEVCTL`):
//   `detach` / `reattach`   let go of the front window's hosted terminals, leaving the views, and attach them again as a
//                           launch does (`window-state` says where each terminal runs)
//   `agents`                what programs that ask to control this instance are up to: each approval in force, the question up
//   `agent-ask <deny|session|quit|always>`     answer the question that is up as that button does
//   `agent-script <hold|deny|session|quit|always>`   what answers questions from now on (`hold`: they wait for `agent-ask`)
//   `agent-grant scripts [quit|always]`        let Scripts in, as an answer to its question does
//   `menubar-icons [prefix]`  the status item's icon in every state, as TIFFs in the control folder
// and the `studio-*` family (`AppController+ControlStudioFamily`). None of it is reachable from the control socket.

import AppKit
import AuxModel
import PlatformModel
import PlatformUI

#if WITH_UNIFFI
    import AuxBridge
    import WorkspaceBridge
#endif

extension AppController {
    func runDevTools(command: String, argument: String) async -> Bool {
        switch command {
        case "detach", "reattach": reply(runHosted(command))
        case "agents", "agent-ask", "agent-script", "agent-grant": reply(runAgents(command, argument.trimmingCharacters(in: .whitespaces)))
        case "menubar-icons": reply(saveStatusIcons(prefix: argument))
        default: return await runStudioFamily(command: command, argument: argument)
        }
        return true
    }

    private func runHosted(_ command: String) -> String {
        #if WITH_UNIFFI
            guard let backend = hub.store.hooks as? RustWorkspaceBackend else { return "error: this window is not the core's" }
            if command == "detach" { return "detached \(backend.detachHosts())" }
            let ids = backend.reattachHosts()
            return ids.isEmpty ? "nothing to reattach" : ids.map(String.init).joined(separator: "\n")
        #else
            return "error: \(command) needs the Rust library"
        #endif
    }

    private func runAgents(_ command: String, _ argument: String) -> String {
        let buttons: [String: AccessButton] = [
            "deny": .deny, "0": .deny, "session": .session, "once": .session, "1": .session, "quit": .quit, "2": .quit, "always": .always, "3": .always,
        ]
        switch command {
        case "agent-ask":
            guard let button = buttons[argument] else { return "error: agent-ask takes deny, session, quit or always, not \(argument)" }
            return aux.accessDriver?.answerPending(button) == true ? "agent-ask \(argument)" : "error: no question is up"
        case "agent-script":
            guard argument == "hold" || buttons[argument] != nil else { return "error: agent-script takes hold, deny, session, quit or always" }
            AccessDriver.script = buttons[argument]
            return "agent-script \(argument)"
        case "agent-grant":
            let words = argument.split(separator: " ").map(String.init)
            guard words.first == "scripts", ["quit", "always"].contains(words.count > 1 ? words[1] : "quit") else {
                return "error: agent-grant takes scripts, then quit or always"
            }
            return grantScripts(always: words.count > 1 && words[1] == "always")
        default:
            return agentsList()
        }
    }

    private func grantScripts(always: Bool) -> String {
        #if WITH_UNIFFI
            guard let desk = (aux.accessBackend as? FFIAccessBackend)?.desk else { return "error: no agent-access desk" }
            return desk.grantScripts(always: always) ? "agent-grant scripts \(always ? "always" : "quit")" : "error: the approval was not stored"
        #else
            return "error: agent-grant needs the Rust library"
        #endif
    }

    /// `approved <stored|memory>:<id> <label> <how> caps=…` per approval, then the question that is up and how many wait.
    private func agentsList() -> String {
        var lines = ["script \(AccessDriver.script.map { "\($0)" } ?? "hold")"]
        #if WITH_UNIFFI
            if let desk = (aux.accessBackend as? FFIAccessBackend)?.desk {
                lines += desk.approvals().map {
                    "approved \($0.stored ? "stored" : "memory"):\($0.id) \($0.label) \($0.how) caps=\($0.allowed.joined(separator: ","))"
                }
                lines.append("waiting \(desk.waiting())")
            }
        #endif
        if let driver = aux.accessDriver {
            if let question = driver.asked {
                let offered = AccessDisplay.of(question.request).buttons.map { "\($0)" }.joined(separator: ",")
                lines.append("asking \(question.id) \(question.request.label) shown \(offered)\(question.request.terminal.map { " about=terminal:\($0.name)" } ?? "")")
            } else if let change = driver.askedMode {
                lines.append("asking mode \(change.previous) -> \(change.requested) shown dontKeep,keep")
            }
        }
        return lines.joined(separator: "\n")
    }

    /// GPUI's nine states of the status item's glyph (`status_item/icon.rs` `save_all`), in the theme's colours.
    private func saveStatusIcons(prefix: String) -> String {
        guard let folder = options.devctl else { return "error: no control folder" }
        let prefix = Self.fileName(prefix.trimmingCharacters(in: .whitespaces), default: "status-item")
        let states: [(name: String, filled: Bool, frame: Int?, mark: StatusMark)] = [
            ("idle", false, nil, .none), ("working", true, nil, .none), ("working-0", true, 0, .none), ("working-1", true, 1, .none),
            ("working-2", true, 2, .none), ("working-3", true, 3, .none), ("needs-you", false, nil, .needsYou), ("failed", false, nil, .failed),
            ("finished", false, nil, .finished),
        ]
        let colors = StatusIconImage.Colors(style: .themed())
        var lines: [String] = []
        for state in states {
            let image = StatusIconImage.image(filled: state.filled, frame: state.frame, mark: state.mark, colors: colors, description: state.name)
            let file = folder.appendingPathComponent("\(prefix)-\(state.name).tiff")
            guard let data = image.tiffRepresentation, (try? data.write(to: file, options: .atomic)) != nil else {
                return "menubar-icons failed: no TIFF for \(state.name)"
            }
            lines.append("saved \(file.path)")
        }
        return lines.joined(separator: "\n")
    }
}

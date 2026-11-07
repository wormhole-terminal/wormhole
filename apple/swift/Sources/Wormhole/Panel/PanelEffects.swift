// What a project's action, prompt or layout asks of the window, performed. Rust decides what trust allows and says it as
// effects (`PanelEffect`); the window owns the terminals, so typing, pasting, opening a link and making terminals are
// done here, and what was refused is said on the notice chip's channel (VoiceOver and the control folder's log).

#if WITH_UNIFFI
    import AppKit
    import FilesBridge
    import Foundation
    import TerminalMetal
    import WindowUI
    import WorkspaceModel

    @MainActor
    final class PanelEffects: PanelEffectPerforming {
        /// Set once the window exists; effects before that have nowhere to go.
        weak var hub: WindowHub?
        var log: (String) -> Void = { _ in }

        func perform(_ effects: [PanelEffect]) async {
            for effect in effects { await perform(effect) }
        }

        func problem(_ text: String) async {
            log("panel: \(text)")
            hub?.panel.host.notify(text)
        }

        private func perform(_ effect: PanelEffect) async {
            switch effect {
            case .type(let command, let place): terminal(for: place)?.source.write(Data((command + "\r").utf8))
            case .paste(let text, let submit, let place):
                guard let source = terminal(for: place)?.source else { return }
                source.paste(text)
                if submit {
                    // The program takes the paste first; Return a moment later is what the person would press.
                    try? await Task.sleep(for: .milliseconds(120))
                    source.write(Data("\r".utf8))
                }
            case .open(let url): NSWorkspace.shared.open(url)
            case .openTerminals(let terminals): terminals.forEach(open)
            }
        }

        private func open(_ spec: NewTerminal) {
            guard let hub else { return }
            let id = existing(named: spec.name) ?? make(in: spec.cwd, named: spec.name)
            guard let id else { return }
            if spec.focus { hub.send(.select(id)) }
            if let run = spec.run, let view = hub.terminals.view(for: id) { view.source.write(Data((run + "\r").utf8)) }
        }

        private func terminal(for place: EffectPlace) -> MetalTerminalView? {
            guard let hub else { return nil }
            let id: TerminalID?
            switch place {
            case .current: id = hub.state.selected
            case .new(let cwd): id = make(in: cwd, named: nil)
            case .named(let name, let cwd): id = existing(named: name) ?? make(in: cwd, named: name)
            }
            return id.flatMap { hub.terminals.view(for: $0) }
        }

        private func existing(named name: String) -> TerminalID? {
            hub?.state.all.first { $0.title == name }?.id
        }

        private func make(in cwd: String?, named name: String?) -> TerminalID? {
            guard let hub else { return nil }
            hub.send(cwd.map { .newTerminalIn($0) } ?? .newTerminal)
            guard let id = hub.state.selected else { return nil }
            if let name { hub.send(.rename(id, name)) }
            return id
        }
    }
#endif

// The terminal's side of what the right panel asks: a next step or a note typed at the prompt, a paste, the keys given
// back, a project's terminal opened. The selected terminal's view is the window's, so the hub answers for it.

import AppKit
import Foundation
import TerminalCore
import WorkspaceModel

extension WindowHub {
    func connect(_ host: SystemPanelHost) {
        host.chosenApp = { [unowned self] in openIn.choice().app }
        // Pasted as the GPUI app does (`SendText`): brackets where the program asked, so an agent takes a note as text.
        host.terminal.send = { [unowned self] text in selectedSource?.paste(text) }
        host.terminal.submit = { [unowned self] in selectedSource?.write(Data("\r".utf8)) }
        host.terminal.takePrompt = { [unowned self] in selectedSource?.grabPrompt() }
        host.terminal.paste = { [unowned self] text in
            // A program without bracketed paste would run each line: the panel copies the text instead.
            guard let source = selectedSource, source.takesPasteAsText else { return false }
            source.paste(text)
            return true
        }
        host.terminal.say = { [unowned self] in notices.post($0) }
        host.terminal.showCommands = { [unowned self] in onShowProjectCommands?() }
        host.terminal.showTab = { [unowned self] name in PanelTab(rawValue: name).map { send(.showPanelTab($0)) } }
        host.terminal.offersTab = { [unowned self] name in panelTabs.contains { $0.rawValue == name } }
        host.terminal.apps = { [unowned self] in openIn.apps() }
        host.launches = { [unowned self] in launchesApps }
        host.terminal.focus = { [unowned self] in
            guard let id = state.selected, let view = terminals.existing(id) else { return }
            view.window?.makeFirstResponder(view)
        }
        host.terminal.open = { [unowned self] folder, command in
            send(.newTerminalIn(folder))
            if let command { selectedSource?.write(Data((command + "\r").utf8)) }
        }
    }

    private var selectedSource: (any FrameSource)? {
        state.selected.flatMap { terminals.view(for: $0) }?.source
    }
}

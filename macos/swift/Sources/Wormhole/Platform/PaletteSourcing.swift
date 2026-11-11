// What the command palette and the shortcut sheet ask this window for (`PaletteSources`): the core's actions for the
// window as the menus see it, the settings index's finds with their values, the commits of the selected terminal's
// branch, and what running a result does. Everything is looked up when asked; nothing here keeps a copy.

import AppKit
import DesignSystem
import PaletteModel
import PlatformModel
import SettingsModel
import WindowUI
import WorkspaceModel

extension AppController {
    func paletteSources(for session: WindowSession) -> PaletteSources {
        PaletteSources(
            actions: { [weak self, weak session] in
                guard let self, let session else { return ActionList() }
                return ListedKeymap.actions(self.actionFacts(session))
            },
            settings: { [weak self] in self?.settingsModel.find($0).compactMap { self?.paletteSetting($0.entry.id, match: $0) } ?? [] },
            setting: { [weak self] in self?.paletteSetting($0, match: nil) },
            commits: { [weak session] in
                (session?.hub.panel.history.model.commits ?? []).map {
                    PaletteCommit(sha: $0.sha, short: $0.short, subject: $0.subject, author: $0.author)
                }
            },
            profiles: { [weak self, weak session] in
                guard let self, let session else { return [] }
                return self.profileList(for: session)().map { PaletteProfile(name: $0.name, isShown: $0.current, isHere: $0.here) }
            },
            perform: { [weak self] in self?.router.run(ActionID(rawValue: $0)) },
            reveal: { [weak self, weak session] target in
                switch target {
                case .terminal(let id): self?.show(terminal: id)
                case .folder(let path): self?.openProject((path as NSString).expandingTildeInPath)
                case .projectAction(let id): self?.router.dispatch(.runProjectAction(id))
                case .newTerminalWith(let key): self?.newTerminal(shellKey: key, in: session)
                case .setting(let id):
                    self?.showSettings()
                    self?.settingsModel.reveal(id)
                case .profile(let command, let index):
                    // The menus' own commands by index: the router carries them out.
                    let menu: MenuCommand =
                        switch command {
                        case .switchTo: .switchProfile(index)
                        case .openWindowIn: .openInProfile(index)
                        case .moveWindowTo: .moveToProfile(index)
                        }
                    self?.router.dispatch(menu)
                case .commit(let sha):
                    session?.hub.send(.showPanelTab(.history))
                    session?.hub.panel.history.show(sha)
                }
            },
            hiddenActions: { [weak self, weak session] in
                guard let self, let session else { return [] }
                return self.actionFacts(session).hidden
            },
            hotkey: { [weak self] in
                // The global key as `config.json` writes it; the sheet's footer writes it in Apple's notation.
                self?.settingsModel.value("hotkey").string.flatMap { try? KeySpec(parsing: $0).glyphText }
            },
            copySettingKey: { [weak self] id in
                guard let key = self?.settingsModel.schema.entry(id)?.entry.keys.first else { return }
                Pasteboard.setText(key)
                self?.hub.notices.post(.copied(.settingKey))
            },
            terminals: { [weak self, weak session] in self?.paletteTerminals(own: session) ?? [] },
            project: { [weak session] in session.flatMap { PaletteProject($0.hub.panel.project.state) } },
            shells: { [weak self] in
                self?.settingsModel.host.shellChoices().filter { !$0.isDefault }
                    .map { PaletteShell(key: $0.key, name: $0.name, commandLine: $0.commandLine) } ?? []
            },
            recentFolders: { [weak self] in self?.recents.roots ?? [] },
            openInNewWindow: { [weak self, weak session] in self?.openInNewWindow($0, from: session) },
            // The one close entry point: it asks while something runs, and closes the window when it was the last terminal.
            closeTerminal: { [weak self] id in self?.closeTerminal(id) })
    }

    /// The menus' facts about the window, as the palette's action list wants them: what is flipped, greyed or hidden.
    private func actionFacts(_ session: WindowSession) -> ActionFacts {
        let state = menuState(for: session.hub)
        return ActionFacts(
            off: state.off.map(\.rawValue), disabled: state.disabled.map(\.rawValue), hidden: state.hidden.map(\.rawValue),
            isSplit: state.split, isDebug: state.debug, isPersistent: state.persistent)
    }

    private func paletteSetting(_ id: String, match hit: SearchHit?) -> PaletteSetting? {
        guard let found = settingsModel.schema.entry(id), let key = found.entry.keys.first else { return nil }
        return PaletteSetting(
            id: id, key: key, label: found.entry.label, section: found.section.title, icon: Icon(section: found.section.id).rawValue,
            value: SettingValueText.of(key: key, settingsModel.value(key)),
            match: hit.map { Match(score: $0.score, ranges: $0.labelRange.map { [$0] } ?? []) })
    }
}

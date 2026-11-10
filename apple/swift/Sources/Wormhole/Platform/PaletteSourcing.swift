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
                return ListedKeymap.actions(self.actionFacts(session.hub))
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
                return self.actionFacts(session.hub).hidden
            },
            hotkey: { [weak self] in
                // The global key as `config.json` writes it; the sheet's footer writes it in Apple's notation.
                self?.settingsModel.value("hotkey").string.flatMap { try? KeySpec(parsing: $0).glyphText }
            },
            copySettingKey: { [weak self] id in
                guard let key = self?.settingsModel.schema.entry(id)?.entry.keys.first else { return }
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(key, forType: .string)
            })
    }

    /// The menus' facts about the window, as the palette's action list wants them: what is flipped, greyed or hidden.
    private func actionFacts(_ hub: WindowHub) -> ActionFacts {
        let state = hub.menuState(
            look: look, isFullScreen: window?.styleMask.contains(.fullScreen) ?? false, implemented: router.implemented,
            flagsOn: Set(settingsModel.host.flags().filter(\.on).map(\.key)))
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

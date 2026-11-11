// The menu bar for the window in front: `MenuStateMaker`'s state through `AppMenuBuilder`, put up again whenever
// anything it read changes (observation, not a timer), and the one callback that carries every choice to the
// action table. In a test instance (`--background`) the bar is built but never installed: it would replace the
// menus of the app in front.

import AppKit
import Observation
import PlatformModel
import PlatformUI
import WindowUI

@MainActor
final class MenuController {
    private let builder: AppMenuBuilder
    private let provider: any MenuProviding
    private unowned let app: AppController
    private var standard: [MenuCommand: Selector]
    private var shown = MenuState()

    init(app: AppController) {
        self.app = app
        // Copy goes through the app so a copy of the terminal's selection can say what it copied (the chip).
        standard = StandardActions.selectors
        standard[.action("copy")] = nil
        provider = RekeyedMenus(base: OutputMenuItems(base: PlatformSetup.menus(app.options)))
        builder = AppMenuBuilder(builder: provider, standard: standard)
        builder.dispatcher.handler = { [weak app] command in app?.router.dispatch(command) }
    }

    /// Puts the bar up and keeps it in step with the window.
    func start() {
        refresh()
    }

    /// The window in front changed, or one came or went: the bar follows the one commands now act on.
    func windowsChanged() { refresh() }

    private func refresh() {
        let state = withObservationTracking {
            app.hub.menuState(
                look: app.look, isFullScreen: app.window?.styleMask.contains(.fullScreen) ?? false,
                implemented: app.router.implemented, flagsOn: Set(app.settingsModel.host.flags().filter(\.on).map(\.key)))
        } onChange: { [weak self] in
            Task { @MainActor in self?.refresh() }
        }
        shown = state
        if app.policy.touchesSystem { builder.update(state) }
    }

    /// The bar as text, one item per line with its key and marks (the control folder's `menus`).
    func dump() -> String { MenuDump.text(provider.menus(for: shown)) }
}

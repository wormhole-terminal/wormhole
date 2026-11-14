// text-file: a test driver's replies on the control folder, not interface text
// The control folder's read-outs of what a view holds (`WORMHOLE_DEVCTL`), each the text of the view's own model in
// `DesignSystem.Readout`'s shape (the end-to-end tests' `Readout.swift` parses it):
//   `settings-page`             the Settings page on show: nav marks, banners, each row's label, value, marks and notes
//   `theme-cards [filter]`      Appearance's theme cards, shelf by shelf, with the filter box holding `filter`
//   `status-items`              the status bar's items at the window's width (whether or not the bar is on: `status-bar on`)
//   `tab-bar` / `tab-click <n|+>`  the tab bar's tabs; a click on the n-th (from 1) or on its +
//   `switcher [query]`          the group switcher's cards and footer, or what its search lists for `query`
//   `switcher open` / `switcher-press new-group|sidebar|card <n>|add <n>`  open the bar's switcher; its buttons and cards
//   `onboarding-state`          the first run's step, its card, buttons, places, agents, trust card and primer
//   `onboarding-pick <path>`    a folder dropped on the folder step (`OnboardingSession.pick`)
//   `onboarding-press back|main|skip|trust|later|theme <name>`  the card's buttons (main: Continue, or Start on the last
//                               step), the trust card's, a Look step theme card
// GPUI's devctl has none of these; `status-bar` is the bar's switch already (`AppController+Look`), so its items are
// `status-items`. The right panel's are `AppController+ControlPanelReadouts`.

import AppKit
import AuxModel
import AuxWindows
import DesignSystem
import Foundation
import PanesUI
import SettingsUI
import WindowUI

extension AppController {
    /// False when `command` is not one of these.
    func runReadouts(command: String, argument: String) -> Bool {
        let argument = argument.trimmingCharacters(in: .whitespaces)
        switch command {
        case "settings-page": reply(SettingsPageReadout.text(settingsModel))
        case "theme-cards": reply(ThemeCardsReadout.text(settingsModel, filter: argument))
        case "status-items": reply(hub.statusItems(look: look, width: window?.contentView?.bounds.width ?? 0))
        case "tab-bar": reply(hub.tabBarListing(look: look))
        case "tab-click": reply(hub.tabClick(argument, look: look).map { "error: \($0)" } ?? "tab-click \(argument)")
        case "switcher":
            if argument == "open" {
                hub.openSwitcher()
                reply("switcher open")
            } else {
                reply(hub.switcherListing(look: look, query: argument))
            }
        case "switcher-press": reply(hub.switcherPress(argument, look: look).map { "error: \($0)" } ?? "switcher-press \(argument)")
        case "onboarding-state", "onboarding-pick", "onboarding-press": reply(onboarding(command, argument))
        default: return runPanelReadouts(command: command, argument: argument)
        }
        return true
    }

    /// The first run's card: read, a folder dropped on it, its buttons (`StepFooter`, `StepCard`, `TrustCardView`, a Look card).
    private func onboarding(_ command: String, _ argument: String) -> String {
        guard let session = aux.onboarding?.session else { return "error: the intro window is not open (`onboarding-step folder`)" }
        switch command {
        case "onboarding-state": return OnboardingReadout.text(session)
        case "onboarding-pick":
            guard argument.hasPrefix("/") else { return "error: onboarding-pick <absolute path>" }
            session.pick(URL(fileURLWithPath: argument))
            return "onboarding-pick \(argument)"
        default: break
        }
        switch argument {
        case "back": session.send(.back)
        // The main button: Continue, or Start on the last step.
        case "main": session.send(session.step.isLast ? .start : .continue)
        case "skip": session.send(.skip)
        // Trust with Touch ID (asked through the instance's `WORMHOLE_AUTH`) and Later.
        case "trust":
            guard let root = session.trustCard?.root else { return "error: no trust card" }
            session.send(.trust(root))
        case "later": session.laterOnTrust()
        default:
            // A Look step card by its theme's name (`LookStep`: the card's own action).
            guard argument.hasPrefix("theme "), let theme = ThemeStore.shared.library.themes.first(where: { $0.name == String(argument.dropFirst(6)) }) else {
                return "error: onboarding-press back|main|skip|trust|later|theme <name>"
            }
            ThemeStore.shared.setTheme(theme.id)
        }
        return "onboarding-press \(argument)"
    }
}

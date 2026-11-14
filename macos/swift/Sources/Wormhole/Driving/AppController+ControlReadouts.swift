// text-file: a test driver's replies on the control folder, not interface text
// The control folder's read-outs of what a view holds (`WORMHOLE_DEVCTL`), each the text of the view's own model in
// `DesignSystem.Readout`'s shape (the end-to-end tests' `Readout.swift` parses it):
//   `settings-page`             the Settings page on show: nav marks, banners, each row's label, value, marks and notes
//   `theme-cards [filter]`      Appearance's theme cards, shelf by shelf, with the filter box holding `filter`
//   `status-items`              the status bar's items at the window's width (whether or not the bar is on: `status-bar on`)
//   `tab-bar` / `tab-click <n|+>`  the tab bar's tabs; a click on the n-th (from 1) or on its +
//   `switcher [query]`          the group switcher's cards and footer, or what its search lists for `query`
//   `onboarding-state`          the first run's step, its card, buttons, places, agents, trust card and primer
//   `onboarding-pick <path>`    a folder dropped on the folder step (`OnboardingSession.pick`)
//   `onboarding-press back|main|skip`  the card's buttons (main: Continue, or Start on the last step)
// GPUI's devctl has none of these; `status-bar` is the bar's switch already (`AppController+Look`), so its items are
// `status-items`. The right panel's are `AppController+ControlPanelReadouts`.

import AppKit
import AuxModel
import AuxWindows
import Foundation
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
        case "switcher": reply(hub.switcherListing(look: look, query: argument))
        case "onboarding-state":
            guard let session = aux.onboarding?.session else { reply("error: the intro window is not open (`onboarding-step folder`)"); return true }
            reply(OnboardingReadout.text(session))
        case "onboarding-pick":
            guard let session = aux.onboarding?.session else { reply("error: the intro window is not open (`onboarding-step folder`)"); return true }
            guard argument.hasPrefix("/") else { reply("error: onboarding-pick <absolute path>"); return true }
            session.pick(URL(fileURLWithPath: argument))
            reply("onboarding-pick \(argument)")
        case "onboarding-press":
            // The card's buttons as `StepFooter` and `StepCard` send them: Back, the main one (Continue, or Start on the last
            // step), Skip setup.
            guard let session = aux.onboarding?.session else { reply("error: the intro window is not open (`onboarding-step folder`)"); return true }
            switch argument {
            case "back": session.send(.back)
            case "main": session.send(session.step.isLast ? .start : .continue)
            case "skip": session.send(.skip)
            default: reply("error: onboarding-press back|main|skip"); return true
            }
            reply("onboarding-press \(argument)")
        default: return runPanelReadouts(command: command, argument: argument)
        }
        return true
    }
}

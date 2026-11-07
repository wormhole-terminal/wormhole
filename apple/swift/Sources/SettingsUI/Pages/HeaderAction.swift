// What sits at the right of a page's title (`header_action`): Coding agents' Rescan PATH, Import's Scan again,
// Profile's New profile. They ask the host, which refreshes the page's records.

import SettingsModel
import SwiftUI

struct HeaderAction: View {
    let model: SettingsModel
    let section: String

    var body: some View {
        switch section {
        case "agents": button(SettingsText.rescanPath, "arrow.clockwise", .rescanAgents)
        case "import": button(SettingsText.scanAgain, "arrow.clockwise", .scanImports)
        case "profile": button(SettingsText.newProfile, "plus", .newProfile)
        default: EmptyView()
        }
    }

    private func button(_ title: String, _ symbol: String, _ action: SettingsAction) -> some View {
        ActionButton(title: title, symbol: symbol) { model.host.perform(action) }
    }
}

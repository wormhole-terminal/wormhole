// The right panel's content: the Panel modules' views, reading through the controller. Kept in one file so the window
// knows the panel only through this seam. A diff or file opened in a tab is laid over the terminal instead of in
// the panel (`TerminalColumn`).
// `PanelModel` has a tab type of its own (it knows nothing of the window), so it is matched by name here.

import FileIconTheme
import PanelModel
import PanelUI
import SwiftUI

struct PanelBody: View {
    let tabName: String
    let controller: PanelController

    var body: some View {
        PanelContentView(controller: controller)
            .environment(\.fileIcons, Self.icons)
            .onAppear { sync() }
            .onChange(of: tabName) { sync() }
    }

    /// The icon theme in force, one value the windows share (the cover over the terminal draws file icons too): the app
    /// sets it from the `icon_theme` setting.
    static let icons = FileIconSource.shared

    private func sync() { controller.tab = PanelModel.PanelTab(rawValue: tabName) ?? .changes }
}

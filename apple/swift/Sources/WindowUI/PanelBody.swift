// The right panel's content: the Panel modules' views, reading through the controller. Kept in one file so the window
// knows the panel only through this seam; the backends are the mock ones until the Rust core arrives through the FFI.
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

    /// The icon theme bundled with the app, loaded once.
    private static let icons = BundledIconTheme()

    private func sync() { controller.tab = PanelModel.PanelTab(rawValue: tabName) ?? .changes }
}

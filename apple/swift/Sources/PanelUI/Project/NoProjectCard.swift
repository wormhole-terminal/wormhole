// A folder with no `.wormhole/` (`render_no_project`): what one is, and the button that makes it.

import DesignSystem
import PanelModel
import SwiftUI

struct NoProjectCard: View {
    let store: ProjectStore
    @Environment(\.tokens) private var tokens
    @Environment(\.panelHost) private var host

    var body: some View {
        PanelCard {
            CardBand(title: panelText("project-folder", "Project folder"))
            VStack(alignment: .leading, spacing: Space._3) {
                Text(panelText("project-none", "This terminal's folder has no .wormhole/ project folder."))
                Text(panelText("project-none-about", "A committed .wormhole/ can hold project settings, a theme, actions, prompts, a terminal layout and shared notes. Every part is optional, and you choose which ones to use."))
                VStack(alignment: .leading, spacing: Space._2) {
                    NoticeButton(title: panelText("project-create", "Create project folder"), primary: true, busy: store.folder == nil, action: store.create)
                    Text(target).font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
                }
                if let error = store.error { Text(error).font(TextRole.caption.font).foregroundStyle(tokens.statusDangerText) }
            }
            .font(TextRole.body.font).foregroundStyle(tokens.textMuted)
            .padding(.horizontal, PanelMetrics.inset).padding(.vertical, Space._2)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private var target: String {
        guard let folder = store.folder else { return panelText("project-no-folder", "This terminal's folder isn't known yet.") }
        let home = host?.homeFolder
        let shown = home.flatMap { folder.hasPrefix($0) ? "~" + folder.dropFirst($0.count) : nil } ?? folder
        return panelText("project-creates", ["folder": .text(shown)], "Creates \(shown)/.wormhole/")
    }
}

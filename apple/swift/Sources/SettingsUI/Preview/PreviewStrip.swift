// The strip at the top of a page that has a scene (`render_preview_strip`): a quiet head (the label and the
// scene's name), then one flat rounded stage with the scene centred on it, and under it a caption where the
// picture needs one. The same height on every page. Scenes draw only themselves.

import DesignSystem
import SettingsModel
import SwiftUI

struct PreviewStrip: View {
    let model: SettingsModel
    let scene: PreviewScene
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            HStack(spacing: Space._2) {
                Text(SettingsText.preview).font(TextRole.label.font).foregroundStyle(tokens.textMuted)
                Text(scene.title).font(.system(size: TextRole.label.size)).foregroundStyle(tokens.textFaint)
            }
            .frame(height: Metric.control)
            .padding(.horizontal, Space._4)
            ZStack {
                RoundedRectangle(cornerRadius: tokens.metrics.radiusLg).fill(tokens.settingsGroup)
                sceneView.padding(SettingsMetric.stageInset)
            }
            .frame(height: SettingsMetric.previewHeight - Metric.control - Space._2)
            .clipShape(RoundedRectangle(cornerRadius: tokens.metrics.radiusLg))
            if let caption = scene.caption {
                Text(caption).font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textMuted)
                    .lineLimit(1).padding(.horizontal, Space._4)
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(scene.spoken)
    }

    @ViewBuilder private var sceneView: some View {
        switch scene {
        case .window: WindowScene(model: model)
        case .fonts: FontsScene(model: model)
        case .sidebar: SidebarScene(model: model, withAgents: false)
        case .agents: SidebarScene(model: model, withAgents: true)
        case .notifications: NotificationsScene(model: model)
        case .menuBar: MenuBarScene(model: model)
        case .hotkey: HotkeyScene(model: model)
        case .git: GitScene(model: model)
        }
    }
}

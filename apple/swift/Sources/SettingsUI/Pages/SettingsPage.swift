// A section's page (`render_section`): its title and intro, the preview strip when it has a scene, then its
// groups, each a card of rows. A row whose `visibleWhen` does not hold is not there, and a group left empty is
// not either. The scroll view jumps to a row a search result named.

import DesignSystem
import SettingsModel
import SwiftUI

struct SettingsPage: View {
    let model: SettingsModel
    let section: SettingsSchema.Section

    var body: some View {
        PageScaffold(title: section.title, intro: section.intro, revealed: model.revealed, clearReveal: model.clearReveal, action: { HeaderAction(model: model, section: section.id) }) {
            if let scene = PreviewScene(section: section.id) { PreviewStrip(model: model, scene: scene) }
            ForEach(Array(section.groups.enumerated()), id: \.offset) { _, group in
                let rows = group.entries.filter { $0.isShown(values: model.values) && $0.control.layout != .strip }
                if !rows.isEmpty {
                    SettingGroup(title: group.title, aside: group.aside, blurb: group.blurb) {
                        ForEach(rows) { EntryRowView(model: model, entry: $0) }
                    }
                }
            }
        }
    }
}

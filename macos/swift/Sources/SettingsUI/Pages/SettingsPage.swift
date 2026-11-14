// A section's page (`render_section`): its title and intro, the preview strip when it has a scene, then its
// groups, each a card of rows. A row whose `visibleWhen` does not hold is not there, and a group left empty is
// not either. The scroll view jumps to a row a search result named.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct SettingsPage: View {
    let model: SettingsModel
    let section: SettingsSchema.Section

    var body: some View {
        // A section opened from a search keeps the matches marked until the search changes.
        let marks = Dictionary(model.hits.map { ($0.entry.id, $0) }, uniquingKeysWith: { first, _ in first })
        PageScaffold(title: section.title, intro: section.intro, revealed: model.revealed, clearReveal: model.clearReveal, action: { HeaderAction(model: model, section: section.id) }) {
            ForEach(Array(PageBanner.of(section, model).enumerated()), id: \.offset) { _, banner in self.banner(banner) }
            if let scene = PreviewScene(section: section.id) { PreviewStrip(model: model, scene: scene) }
            ForEach(Array(section.groups.enumerated()), id: \.offset) { _, group in
                let rows = group.entries.filter { $0.isShown(values: model.values) && $0.control.layout != .strip }
                if let bare = rows.first(where: { $0.control.layout == .bare }) {
                    EntryRowView(model: model, entry: bare, found: marks[bare.id])
                } else if !rows.isEmpty {
                    SettingGroup(title: group.title, aside: group.aside, blurb: group.blurb) {
                        ForEach(rows) { EntryRowView(model: model, entry: $0, found: marks[$0.id]) }
                    }
                }
            }
        }
    }

    @ViewBuilder private func banner(_ banner: PageBanner) -> some View {
        switch banner {
        case .zen: ZenNote(leave: { model.leaveZen?() })
        case .preview:
            Banner(kind: banner.kind, text: banner.text) {
                ActionButton(title: Words.text("settings-window-feature-flags", fallback: "Feature flags"), symbol: nil) { model.show("featureFlags") }
            }
        case .broken:
            Banner(kind: banner.kind, text: banner.text) {
                ActionButton(title: SettingsText.open, symbol: nil, frameId: "banner.open") { model.host.perform(.openConfig) }
            }
        case .unreadable, .profiles, .records: Banner(kind: banner.kind, text: banner.text)
        }
    }
}

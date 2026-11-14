// Open in…: a button with the chosen app's icon that opens the terminal's folder in it, and a narrow chevron
// beside it that lists the other apps found on this Mac with the chosen one checked (`render_title_buttons`: the shared
// button `narrow(CHEVRON_W)` with `menu_chevron`).

import AppKit
import DesignSystem
import PanesModel
import SwiftUI

struct OpenInButtons: View {
    let folder: String
    let backend: any OpenInBackend
    let icon: (OpenInApp) -> NSImage?

    var body: some View {
        let open = OpenInMenu(folder: folder, backend: backend), choice = open.choice
        HStack(spacing: 0) {
            // An app's own icon keeps its colours and has no motion of the glyph's (`with_drawing`).
            IconButton(label: tooltip(choice), action: open.click) { _ in AppIcon(app: choice.app, image: icon(choice.app), size: Metric.titleGlyph) }
                .inTitleRow()
            IconMenuButton(lucide: .chevronDown, label: PanesText.openInAnother) {
                Section(PanesText.openFolderIn) {
                    ForEach(open.items, id: \.app.bundle) { item in
                        Toggle(isOn: Binding(get: { item.checked }, set: { _ in open.pick(item.app) })) {
                            Label { Text(item.app.name) } icon: { AppIcon(app: item.app, image: icon(item.app), size: Metric.titleGlyph) }
                        }
                    }
                }
            }
            .narrow(TitleMetrics.chevronWidth)
            .chevron()
            .frame(width: TitleMetrics.chevronWidth + TitleMetrics.afterChevron, alignment: .leading)
        }
    }

    private func tooltip(_ choice: OpenInChoice) -> String {
        choice.missing.map { PanesText.appMissing($0, choice.app.name) } ?? PanesText.openFolderInApp(choice.app.name)
    }
}

/// An app's own icon, or its monogram on a neutral square when the system has none.
struct AppIcon: View {
    let app: OpenInApp
    let image: NSImage?
    let size: CGFloat
    @Environment(\.tokens) private var tokens

    var body: some View {
        if let image {
            Image(nsImage: image).resizable().frame(width: size, height: size)
        } else {
            Text(app.monogram).font(.system(size: size * 0.5, weight: .semibold)).foregroundStyle(tokens.textPrimary)
                .frame(width: size, height: size)
                .background(tokens.stateActive, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
        }
    }
}

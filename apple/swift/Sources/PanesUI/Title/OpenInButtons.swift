// Open in…: a button with the chosen app's icon that opens the terminal's folder in it, and a narrow chevron
// beside it that lists the other apps found on this Mac with the chosen one checked (`render_title_buttons`).

import AppKit
import DesignSystem
import PanesModel
import SwiftUI

struct OpenInButtons: View {
    let folder: String
    let backend: any OpenInBackend
    let icon: (OpenInApp) -> NSImage?
    @Environment(\.tokens) private var tokens

    var body: some View {
        let open = OpenInMenu(folder: folder, backend: backend), choice = open.choice
        HStack(spacing: 0) {
            TitleIconButton(label: tooltip(choice), action: open.click) {
                AppIcon(app: choice.app, image: icon(choice.app), size: Metric.titleGlyph)
            }
            Menu {
                Section(PanesText.openFolderIn) {
                    ForEach(open.items, id: \.app.bundle) { item in
                        Toggle(isOn: Binding(get: { item.checked }, set: { _ in open.pick(item.app) })) {
                            Label { Text(item.app.name) } icon: { AppIcon(app: item.app, image: icon(item.app), size: Metric.titleGlyph) }
                        }
                    }
                }
            } label: {
                Image(systemName: "chevron.down").font(.system(size: Metric.chevron, weight: .semibold))
                    .frame(width: TitleMetrics.chevronWidth, height: Metric.titleControl)
                    .foregroundStyle(tokens.textMuted)
            }
            .menuStyle(.button).buttonStyle(.plain).menuIndicator(.hidden)
            .frame(width: TitleMetrics.chevronWidth + TitleMetrics.afterChevron, alignment: .leading)
            .help(PanesText.openInAnother).accessibilityLabel(PanesText.openInAnother)
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

// The Studio's title row, 38 high: the traffic lights, "Theme Studio / <name> ▾" (the name is the theme menu),
// what the theme extends, Unsaved; then Light | Dark for a pair, Undo and Redo, Revert to Saved, Export… and
// the Save theme split (`studio/title.rs`).

import DesignSystem
import StudioModel
import SwiftUI

struct TitleBar: View {
    @Bindable var session: StudioSession
    @Binding var showsVersions: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        let document = session.document
        HStack(spacing: Space._2) {
            Text(StudioWords.title).font(TextRole.body.font).foregroundStyle(tokens.textMuted)
            Text("/").font(TextRole.body.font).foregroundStyle(tokens.textFaint)
            if session.renaming { NameField(session: session) } else { ThemeNameMenu(session: session, showsVersions: $showsVersions) }
            Text(basedOn(document)).font(TextRole.caption.font).foregroundStyle(tokens.textFaint).lineLimit(1)
            if document.isDirty {
                Text(StudioWords.unsaved).font(TextRole.caption.font).foregroundStyle(tokens.statusWarningText)
                    .padding(.horizontal, Space._2).frame(height: Metric.badgeHeight)
                    .background(tokens.bannerWarningBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            }
            Spacer(minLength: Space._2)
            AppearancePicker(document: document)
            UndoButtons(document: document)
            Button(StudioWords.revert) { document.revertToSaved() }.buttonStyle(.plain).font(TextRole.body.font)
                .foregroundStyle(document.isDirty ? tokens.textMuted : tokens.textFaint).disabled(!document.isDirty)
            Button { Task { await session.exportJSON() } } label: { Label(StudioWords.export, systemImage: "square.and.arrow.up") }
                .buttonStyle(.bordered).controlSize(.small)
            SaveButton(session: session)
        }
        .padding(.leading, StudioMetrics.lightsEnd).padding(.trailing, Space._3)
        .frame(height: Metric.titleRow)
        .background(tokens.titlebarBackground)
        .hairline(.bottom)
        .accessibilityElement(children: .contain)
    }

    private func basedOn(_ document: StudioDocument) -> String {
        let appearance = document.appearance == .light ? StudioWords.light : StudioWords.dark
        return document.file.extends.map { StudioWords.extends($0, appearance) } ?? StudioWords.ownPalette(appearance)
    }
}

// "Where do you work?": the places a Finder sidebar starts with and the folders worked in lately, "Open folder…",
// and a box to drop a folder on (`steps.rs` `render_folder_step`). Nothing is scanned for repositories.

import AuxModel
import DesignSystem
import SwiftUI
import UniformTypeIdentifiers

struct FolderStep: View {
    let session: OnboardingSession
    @Namespace private var selection
    @Environment(\.tokens) private var tokens
    @State private var dropTargeted = false
    @Environment(\.stepKeys) private var keys

    var body: some View {
        let picked = session.flow.folder
        let onPlace = session.places.contains { $0.url == picked }
        VStack(alignment: .leading, spacing: 0) {
            Rule()
            Row(isSelected: picked != nil && !onPlace, selection: selection, action: { Task { await session.chooseFolder() } }) { _ in
                HStack(alignment: .top, spacing: Space._2) {
                    Image(systemName: "folder").frame(width: Metric.lead).foregroundStyle(tokens.textMuted)
                    VStack(alignment: .leading, spacing: 0) {
                        Text(AuxText.openFolder).font(TextRole.body.font.weight(.medium))
                        Text((onPlace ? nil : picked).map { AuxText.startsIn(Place.tilde($0, home: .homeDirectory)) } ?? AuxText.openFolderDetail)
                            .font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                    }
                    Spacer(minLength: 0)
                    KeyCap(text: "\u{2318}O")
                }
                .padding(.vertical, Space._2)
            }
            .keyboardShortcut(keys ? KeyboardShortcut("o", modifiers: .command) : nil)
            let recents = session.places.filter { $0.kind == .recent }
            if !recents.isEmpty { label(AuxText.recents); ForEach(recents) { place(rowFor: $0, picked: picked) } }
            label(AuxText.places)
            ForEach(session.places.filter { $0.kind != .recent }) { place(rowFor: $0, picked: picked) }
            dropZone
        }
        .dropDestination(for: URL.self) { urls, _ in
            guard let folder = urls.first(where: \.hasDirectoryPath) else { return false }
            session.pick(folder)
            return true
        } isTargeted: { dropTargeted = $0 }
    }

    private func label(_ text: String) -> some View {
        SectionLabel(text).padding(.horizontal, Space._4).padding(.top, Space._1)
    }

    private func place(rowFor place: Place, picked: URL?) -> some View {
        Row(isSelected: picked == place.url, selection: selection, action: { session.pick(place.url) }) { _ in
            HStack(spacing: Space._2) {
                Image(systemName: Self.symbol(place.kind)).frame(width: Metric.lead).foregroundStyle(tokens.textMuted)
                Text(place.name).font(TextRole.body.font)
                Text(place.detail).font(TextRole.caption.font).foregroundStyle(tokens.textFaint).lineLimit(1).truncationMode(.middle)
                Spacer(minLength: 0)
            }
            .frame(minHeight: Metric.row)
        }
        .accessibilityLabel("\(place.name), \(place.detail)")
        .accessibilityAddTraits(picked == place.url ? .isSelected : [])
    }

    private var dropZone: some View {
        HStack(spacing: Space._2) {
            Image(systemName: "folder.badge.plus")
            Text(AuxText.dropFolder)
        }
        .font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
        .frame(maxWidth: .infinity, minHeight: OnboardingMetrics.dropZoneHeight)
        .background(dropTargeted ? tokens.stateHighlight : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
        .overlay {
            RoundedRectangle(cornerRadius: tokens.metrics.radius)
                .strokeBorder(tokens.borderStrong, style: StrokeStyle(lineWidth: tokens.metrics.borderWidth, dash: [4, 3]))
        }
        .padding(.horizontal, Space._4).padding(.top, Space._4).padding(.bottom, Space._4)
    }

    static func symbol(_ kind: Place.Kind) -> String {
        switch kind {
        case .recent: "clock"
        case .home: "house"
        case .desktop: "menubar.dock.rectangle"
        case .documents: "doc.text"
        case .developer: "hammer"
        }
    }
}

extension URL {
    fileprivate static var homeDirectory: URL { FileManager.default.homeDirectoryForCurrentUser }
}

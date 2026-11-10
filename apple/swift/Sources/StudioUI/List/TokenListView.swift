// The token list, on the left: a filter, the tokens in folder bands read like the sidebar, a swatch or shape in
// the lead column, the name in the code font, the value faint on the right. It shows what the document's filter
// keeps, narrowed by the gallery's area too; the selected token that the area leaves out stays in the editor and
// the list says why (`studio/token_list.rs`).

import DesignSystem
import StudioModel
import SwiftUI

struct TokenListView: View {
    @Bindable var document: StudioDocument
    @State private var open: Set<String> = []
    @Namespace private var selection
    @Environment(\.tokens) private var tokens

    var body: some View {
        let counts = document.counts
        let low = document.lowContrastTokens
        VStack(spacing: 0) {
            TokenListHeader(filter: $document.filter, yours: counts.yours, low: counts.low)
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(spacing: 0, pinnedViews: []) {
                        if let note = notInArea {
                            Label(note, systemImage: "info.circle").font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                                .padding(.horizontal, Space._4).padding(.vertical, Space._2).frame(maxWidth: .infinity, alignment: .leading)
                        }
                        ForEach(document.visibleGroups) { group in band(group, low: low) }
                    }
                    .padding(.bottom, Space._4)
                }
                .onChange(of: document.selected) { _, token in
                    guard let token else { return }
                    reveal(token)
                    withAnimation(Motion.layout) { proxy.scrollTo(token, anchor: .center) }
                }
                .onAppear { if let token = document.selected { reveal(token) } }
            }
        }
        .frame(width: StudioMetrics.listWidth)
        .background(tokens.surfacePanel)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Tokens")
    }

    @ViewBuilder private func band(_ group: TokenGroup, low: Set<String>) -> some View {
        let filtering = document.filter.isFiltering
        let isOpen = filtering || open.contains(group.name)
        let size = document.groups.first { $0.name == group.name }?.tokens.count ?? group.tokens.count
        BandView(
            group: group, size: size, isOpen: isOpen, folds: !filtering, lowCount: group.tokens.filter(low.contains).count,
            changed: group.tokens.contains(where: document.isChanged)
        ) {
            withAnimation(Motion.layout) { if !open.insert(group.name).inserted { open.remove(group.name) } }
        }
        if isOpen {
            ForEach(group.tokens, id: \.self) { token in
                TokenRowView(
                    name: token, value: document.row(for: token), isSelected: document.selected == token, isLow: low.contains(token),
                    isChanged: document.isChanged(token), origin: document.origin(of: token), selection: selection
                ) { document.selected = token }
                .id(token)
            }
        }
    }

    /// Opens the band the token is in, so a token chosen in the gallery or at the start is seen.
    private func reveal(_ token: String) {
        if let group = document.groups.first(where: { $0.tokens.contains(token) }) { open.insert(group.name) }
    }

    private var notInArea: String? {
        guard let selected = document.selected, document.filter.area != .all,
            !document.visibleGroups.contains(where: { $0.tokens.contains(selected) })
        else { return nil }
        return StudioWords.notInArea(selected, document.filter.area.rawValue.capitalized)
    }
}

// The gallery, in the middle: every component on the live draft in two columns, which a filter narrows to one
// part of the app (and which narrows the token list too). What the selected token is drawn on is outlined
// (`studio/gallery/view.rs`). Left-column and right-column specimens stand side by side; the wide ones under both.

import DesignSystem
import StudioModel
import SwiftUI

struct GalleryView: View {
    @Bindable var document: StudioDocument
    @Environment(\.tokens) private var tokens

    var body: some View {
        let area = document.filter.area
        let shown = GalleryCatalog.specimens.filter { area == .all || $0.area == area }
        let context = GalleryContext(selected: document.selected) { document.selected = $0 }
        VStack(spacing: 0) {
            AreaChips(area: $document.filter.area, uses: document.selected.map { (document.uses(of: $0).count, $0) })
            ScrollView {
                VStack(alignment: .leading, spacing: Space._6) {
                    HStack(alignment: .top, spacing: StudioMetrics.galleryGap) {
                        column(shown.filter { $0.column == .left }, context)
                        column(shown.filter { $0.column == .right }, context)
                    }
                    column(shown.filter { $0.column == .both }, context)
                }
                .padding(Space._4)
                .animation(Motion.layout, value: shown.map(\.id))
            }
        }
        .frame(maxWidth: .infinity)
        .background(tokens.surfaceWindow)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Gallery")
    }

    private func column(_ specimens: [Specimen], _ context: GalleryContext) -> some View {
        VStack(alignment: .leading, spacing: Space._6) {
            ForEach(specimens) { SpecimenFrame(specimen: $0, context: context) }
        }
        .frame(maxWidth: .infinity, alignment: .topLeading)
    }
}

/// All, Sidebar, Panel, Chrome, Settings, Ambient: the part of the app the page shows.
struct AreaChips: View {
    @Binding var area: StudioArea
    /// How many things the selected token is drawn on, and its name; `nil` with none selected.
    let uses: (count: Int, token: String)?
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._4) {
            Segmented(StudioArea.allCases.map { ($0.rawValue.capitalized, $0) }, selection: $area)
            Spacer(minLength: 0)
            if let uses { usesChip(uses) }
        }
        .padding(.horizontal, Space._4).padding(.vertical, Space._3)
        .accessibilityLabel("Show the part of the app")
    }

    /// "◎ 7 uses of primary", or that nothing draws it directly (`header` in `gallery/view.rs`).
    private func usesChip(_ uses: (count: Int, token: String)) -> some View {
        let words = uses.count == 0 ? "Nothing draws \(uses.token) directly" : "\(uses.count) \(uses.count == 1 ? "use" : "uses") of \(uses.token)"
        return Label(words, systemImage: "scope").font(TextRole.caption.font).lineLimit(1)
            .foregroundStyle(uses.count == 0 ? tokens.textFaint : tokens.textLink)
    }
}

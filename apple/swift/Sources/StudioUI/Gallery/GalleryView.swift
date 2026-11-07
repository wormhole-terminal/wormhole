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
            AreaChips(area: $document.filter.area)
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
    @Environment(\.tokens) private var tokens

    var body: some View {
        Picker("", selection: $area) {
            ForEach(StudioArea.allCases) { Text($0.rawValue.capitalized).tag($0) }
        }
        .pickerStyle(.segmented).labelsHidden().controlSize(.small)
        .padding(.horizontal, Space._4).padding(.vertical, Space._3)
        .hairline(.bottom)
        .accessibilityLabel("Show the part of the app")
    }
}

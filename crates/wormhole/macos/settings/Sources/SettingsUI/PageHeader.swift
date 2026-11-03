import SwiftUI

/// Stable page identity above the scrolling form; the subtitle tells which sidebar group is open.
struct PageHeader: View {
    @Environment(SettingsModel.self) private var m
    @Environment(\.tok) private var t

    private var page: Page? { m.catalog.page(m.page) }
    private var title: String { m.isSearching ? m.w("Search") : m.w(page?.title ?? (m.page == "Changed" ? "Changed" : "")) }
    private var subtitle: String {
        guard !m.isSearching, let page, !page.subpages.isEmpty,
              page.groups.indices.contains(m.shownGroup) else { return "" }
        return m.w(page.groups[m.shownGroup].title)
    }

    var body: some View {
        HStack(spacing: Space.s3) {
            if m.isSearching {
                Image(systemName: "magnifyingglass").font(.title2).foregroundStyle(.secondary)
                    .frame(width: TileSize.side)
            } else if let item = m.catalog.navItems[m.page] {
                Tile(colour: Color(css: item.colour), symbol: item.symbol).accessibilityHidden(true)
            }
            VStack(alignment: .leading, spacing: Space.s1) {
                HStack(spacing: Space.s2) {
                    Text(title).role(.title, weight: .semibold).accessibilityAddTraits(.isHeader)
                    if !m.isSearching, page?.badge == true {
                        Chip(text: m.w("Preview"), fill: t.primary, ink: t.onPrimary)
                    }
                }
                if !subtitle.isEmpty { Text(subtitle).foregroundStyle(.secondary).role(.body) }
            }
            Spacer(minLength: 0)
        }
        .frame(minHeight: Space.headerHeight, alignment: .leading)
    }
}

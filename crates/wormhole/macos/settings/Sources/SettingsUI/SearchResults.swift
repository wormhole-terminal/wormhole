import SwiftUI

struct SettingsSearchHit: Identifiable {
    let page: Page
    let group: Int
    let rows: [Row]
    var id: String { "\(page.id):\(group)" }
}

extension SettingsModel {
    /// Search exactly the rows this build can show, including the app's dynamic rows and translated words.
    /// Every term must match, in any order; the system comparison handles case and diacritics.
    var searchHits: [SettingsSearchHit] {
        let terms = search.split(whereSeparator: \.isWhitespace).map(String.init)
        guard !terms.isEmpty else { return [] }
        return catalog.pages.filter { !store.hidden.contains($0.id) }.flatMap { page in
            page.groups.indices.compactMap { group in
                let context = [w(page.title), w(page.groups[group].title)]
                let matches = rows(page, group).filter { row in
                    guard !row.label.isEmpty else { return false }
                    let words = (context + [w(row.label), w(row.hint), w(row.info)]).joined(separator: " ")
                    return terms.allSatisfy { words.localizedStandardContains($0) }
                }
                return matches.isEmpty ? nil : SettingsSearchHit(page: page, group: group, rows: matches)
            }
        }
    }
}

struct SearchResults: View {
    @Environment(SettingsModel.self) private var m

    var body: some View {
        let hits = m.searchHits
        if hits.isEmpty {
            ContentUnavailableView {
                Label(m.w("No settings match “%@”", m.search), systemImage: "magnifyingglass")
            } description: {
                Text(m.w("Try a different word or choose a section in the sidebar."))
            }
        } else {
            Form {
                ForEach(hits) { hit in
                    Section {
                        ForEach(hit.rows) { RowView(row: $0) }
                    } header: {
                        HStack {
                            Text(m.w(hit.page.title))
                            if !hit.page.groups[hit.group].title.isEmpty {
                                Image(systemName: "chevron.right").imageScale(.small).accessibilityHidden(true)
                                Text(m.w(hit.page.groups[hit.group].title))
                            }
                            Spacer()
                            Button {
                                m.go("\(hit.page.id)/\(hit.group)")
                            } label: {
                                Image(systemName: "arrow.up.forward")
                            }
                            .buttonStyle(.borderless).help(m.w("Show in Settings"))
                            .accessibilityLabel("\(m.w("Show in Settings")): \(m.w(hit.page.title)), \(m.w(hit.page.groups[hit.group].title))")
                        }
                    }
                }
            }
        }
    }
}

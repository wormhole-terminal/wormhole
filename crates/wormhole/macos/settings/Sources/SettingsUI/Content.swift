import SwiftUI

/// The content: a native grouped Form per page (or search results, or Changed).
struct ContentView: View {
    @Environment(SettingsModel.self) var m
    @Environment(\.tok) var t
    var title: String {
        m.search.isEmpty ? (m.page == "Changed" ? m.w("Changed") : m.catalog.page(m.page).map { m.w($0.title) } ?? "") : m.w("Search")
    }
    var body: some View {
        ScrollViewReader { proxy in
            VStack(alignment: .leading, spacing: 0) {
                HStack(spacing: Space.s2) {
                    Text(title).role(.title, weight: .semibold)
                    if m.search.isEmpty, m.catalog.page(m.page)?.badge == true { Chip(text: m.w("Preview"), fill: t.primary, ink: t.onPrimary) }
                }.padding(.horizontal, Space.s6 + Space.s2).padding(.top, 44).padding(.bottom, Space.s1)
                Group {
                    if !m.search.isEmpty { SearchResults() }
                    else if m.page == "Changed" { ChangedPage() }
                    else if let p = m.catalog.page(m.page) { PageView(page: p) }
                }.formStyle(.grouped).scrollContentBackground(.hidden).multilineTextAlignment(.leading)
            }.background(Color(nsColor: .windowBackgroundColor)).ignoresSafeArea(edges: .top)
            .onChange(of: m.scrollTo) { _, v in if let v { withAnimation { proxy.scrollTo(v, anchor: .top) } } }
        }
    }
}

struct PageView: View {
    let page: Page
    @Environment(SettingsModel.self) var m
    @Environment(\.tok) var t
    var body: some View {
        Form {
            if let n = page.notice, !m.store.quiet(page.id) {
                Section {
                    LabeledContent {
                        if !n.2.isEmpty { Btn(n.2) { m.perform(n.2, "\(page.id):") } }
                    } label: {
                        Label { Text(m.w(n.1)).foregroundStyle(.secondary) } icon: { Image(systemName: n.0 == "warn" ? "exclamationmark.triangle.fill" : "info.circle.fill").foregroundStyle(n.0 == "warn" ? t.warning : t.primary) }
                    }
                }
            }
            if !page.preview.isEmpty { PreviewCard(page: page) }
            ForEach(Array(page.groups.enumerated()), id: \.offset) { g, grp in
                Section {
                    ForEach(m.rows(page, g), id: \.key) { RowView(row: $0) }
                } header: { if !grp.title.isEmpty { Text(m.w(grp.title)) } }.id(grp.title)
            }
            if !page.footer.isEmpty {
                Section { HStack(spacing: Space.s2) { Text(m.w(page.footer)).role(.caption).foregroundStyle(.secondary); ForEach(page.footerButtons, id: \.self) { b in Btn(b) { m.perform(b, "\(page.id):") } } } }
            }
        }
    }
}

struct SearchResults: View {
    @Environment(SettingsModel.self) var m
    var body: some View {
        let q = m.search.lowercased()
        let hits = m.catalog.pages.map { p in
            (p, p.groups.flatMap(\.rows).filter { !$0.label.isEmpty && ($0.label + $0.hint + $0.info).lowercased().contains(q) })
        }.filter { !$0.1.isEmpty }
        Form {
            ForEach(hits, id: \.0.id) { p, rows in Section(m.w(p.title)) { ForEach(rows, id: \.key) { RowView(row: $0) } } }
            if hits.isEmpty { Text(m.w("No settings match “%@”", m.search)).foregroundStyle(.secondary) }
        }
    }
}

struct ChangedPage: View {
    @Environment(SettingsModel.self) var m
    @State private var confirming = false
    var body: some View {
        let ch = m.changed
        Form {
            Section {
                LabeledContent {
                    if !ch.isEmpty {
                        HStack(spacing: Space.s2) { Btn("Copy as JSON") { copyJSON(ch.map(\.1)) }; Btn("Reset All…") { confirming = true } }
                    }
                } label: {
                    HStack(spacing: Space.s3) {
                        Tile(colour: Palette.tileSlate, symbol: "arrow.counterclockwise")
                        VStack(alignment: .leading, spacing: 1) {
                            Text(ch.isEmpty ? m.w("Nothing changed") : ch.count == 1 ? m.w("1 setting changed") : m.w("%@ settings changed", String(ch.count)))
                            Text(ch.isEmpty ? m.w("Every setting is at its default") : m.w("Everything else is at its default")).role(.body).foregroundStyle(.secondary)
                        }
                    }
                }
            }
            ForEach(m.catalog.pages.filter { p in ch.contains { $0.0.id == p.id } }) { p in
                Section(m.w(p.title)) { ForEach(ch.filter { $0.0.id == p.id }.map(\.1), id: \.key) { RowView(row: $0, showReset: true) } }
            }
        }.confirmationDialog(ch.count == 1 ? m.w("Reset 1 setting?") : m.w("Reset %@ settings?", String(ch.count)), isPresented: $confirming, titleVisibility: .visible) {
            Button(m.w("Reset"), role: .destructive) { m.store.resetAll() }
        } message: { Text(m.w("They go back to their defaults.")) }
    }

    /// The changed settings as `{ "key": "value" }`, sorted, for pasting into a bug report or another Mac.
    private func copyJSON(_ rows: [Row]) {
        let pairs = Dictionary(rows.map { ($0.key, m.store.value(for: $0.key) ?? "") }, uniquingKeysWith: { a, _ in a })
        guard let data = try? JSONSerialization.data(withJSONObject: pairs, options: [.prettyPrinted, .sortedKeys]),
              let text = String(data: data, encoding: .utf8) else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }
}

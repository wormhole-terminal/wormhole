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
                }.padding(.horizontal, Space.pageInset).padding(.top, Space.s2).padding(.bottom, Space.s1)
                if let banner = m.banner { ConfigBanner(banner: banner) }
                Group {
                    if !m.search.isEmpty { SearchResults() }
                    else if m.page == "Changed" { ChangedPage() }
                    else if let p = m.catalog.page(m.page) { PageView(page: p) }
                }.formStyle(.grouped).scrollContentBackground(.hidden).multilineTextAlignment(.leading)
            }.frame(maxWidth: Space.contentWidth, alignment: .leading).frame(maxWidth: .infinity)
            .background(Color(nsColor: .windowBackgroundColor))
            .onChange(of: m.scrollTo) { _, v in if let v { withAnimation { proxy.scrollTo(v, anchor: .top) } } }
            .sheet(isPresented: Binding(get: { m.sheetDict("new-profile")?["open"] as? Bool ?? false }, set: { if !$0 { m.perform("new_profile_cancel", "Profile:") } })) { NewProfileSheet() }
            .sheet(isPresented: Binding(get: { m.infoSheet != nil }, set: { if !$0 { m.infoSheet = nil } })) { if m.sheetDict(m.infoSheet ?? "")?["kind"] as? String == "compare" { CompareSheet(id: m.infoSheet ?? "") } else { InfoSheet(id: m.infoSheet ?? "") } }
        }
    }
}

/// An explanation the host gives as data: a title, rows of a label over its values (code when marked), an intro and a closing note.
struct InfoSheet: View {
    let id: String
    @Environment(SettingsModel.self) var m
    @Environment(\.dismiss) var dismiss
    var data: [String: Any] { m.sheetDict(id) ?? [:] }
    var body: some View {
        VStack(alignment: .leading, spacing: Space.s3) {
            Text(data["title"] as? String ?? "").font(.headline)
            ForEach(data["intro"] as? [String] ?? [], id: \.self) { Text($0).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true) }
            ForEach(Array((data["rows"] as? [[String: Any]] ?? []).enumerated()), id: \.offset) { _, r in
                Divider()
                VStack(alignment: .leading, spacing: 2) {
                    Text(r["label"] as? String ?? "").role(.body, weight: .semibold)
                    ForEach(r["values"] as? [String] ?? [], id: \.self) { v in
                        if r["code"] as? Bool ?? false { Text(v).role(.body, mono: true).textSelection(.enabled) } else { Text(v) }
                    }
                    if let why = r["why"] as? String, !why.isEmpty { Text(why).role(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true) }
                }.frame(maxWidth: .infinity, alignment: .leading)
            }
            if let note = data["note"] as? String, !note.isEmpty { Text(note).role(.caption).foregroundStyle(.secondary) }
            HStack { Spacer(); Button(data["done"] as? String ?? "") { dismiss() }.keyboardShortcut(.defaultAction) }
        }.padding(Space.s4).frame(width: 480)
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
                        if !n.2.isEmpty { Btn(n.2) { m.perform(snakeID(n.2), "\(page.id):") } }
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
                Section { HStack(spacing: Space.s2) { Text(m.w(page.footer)).role(.caption).foregroundStyle(.secondary); ForEach(page.footerButtons, id: \.self) { b in Btn(b) { footer(snakeID(b)) } } } }
            }
        }
    }
}

extension PageView {
    /// A page's footer button: some only move inside the window (to another page, to an explanation), the rest are the app's.
    func footer(_ id: String) {
        switch id {
        case "open_notifications": m.search = ""; m.page = "Notifications"
        case "how_status_works": m.infoSheet = "status-how"
        default: m.perform(id, "\(page.id):")
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
                            Text(ch.isEmpty ? m.w("Nothing changed") : m.counted("changed", ch.count, "\(ch.count) settings changed"))
                            Text(ch.isEmpty ? m.w("Every setting is at its default") : m.w("Everything else is at its default")).role(.body).foregroundStyle(.secondary)
                        }
                    }
                }
            }
            ForEach(m.catalog.pages.filter { p in ch.contains { $0.0.id == p.id } }) { p in
                Section(m.w(p.title)) { ForEach(ch.filter { $0.0.id == p.id }.map(\.1), id: \.key) { RowView(row: $0, showReset: true) } }
            }
        }.confirmationDialog(m.counted("reset", ch.count, "Reset \(ch.count) settings?"), isPresented: $confirming, titleVisibility: .visible) {
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

/// A problem with the app's own config file, above the page (a warning, with the way to the file when there is one).
struct ConfigBanner: View {
    let banner: [String: String]
    @Environment(SettingsModel.self) var m
    @Environment(\.tok) var t
    var body: some View {
        HStack(spacing: Space.s2) {
            Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(t.warning)
            Text(banner["text"] ?? "").foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            Spacer(minLength: 0)
            if let action = banner["action"], !action.isEmpty { Button(banner["button"] ?? "") { m.perform(action, "Advanced:") }.buttonStyle(.bordered) }
        }.padding(.horizontal, Space.pageInset).padding(.vertical, Space.s2)
    }
}

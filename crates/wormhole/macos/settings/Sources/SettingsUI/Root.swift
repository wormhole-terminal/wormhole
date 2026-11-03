import SwiftUI
import AppKit

/// The Settings window's content. A host builds it with its own `SettingsStore` (and optionally catalog and words).
public struct SettingsView: View {
    @State private var model: SettingsModel
    public init(store: any SettingsStore, catalog: SettingsCatalog = .standard, words: Words = Words(), page: String = "Terminal") {
        _model = State(initialValue: MainActor.assumeIsolated { SettingsModel(store: store, catalog: catalog, words: words, page: page) })
    }

    /// A host that keeps the model itself (to move the page of a window that is already open).
    init(model: SettingsModel) { _model = State(initialValue: model) }

    public var body: some View {
        Inner().environment(model).preferredColorScheme(model.colourScheme)
    }

    private struct Inner: View {
        @Environment(SettingsModel.self) var m
        @Environment(\.colorScheme) var scheme
        var body: some View {
            @Bindable var s = m
            let t = Tok(dark: scheme == .dark, accentOverride: m.accent)
            NavigationSplitView {
                SidebarView().navigationSplitViewColumnWidth(Space.sidebarWidth)
            } detail: {
                ContentView().toolbarBackground(.hidden, for: .windowToolbar)
            }
            .searchable(text: $s.search, placement: .sidebar, prompt: "Search")
            .environment(\.tok, t).tint(t.primary)
        }
    }
}

/// The sidebar: a native source list with the board's nav tiles. Under the open page, its groups scroll to.
struct SidebarView: View {
    @Environment(SettingsModel.self) var m
    @Environment(\.tok) var t
    @State private var activeGroup: String?

    var body: some View {
        @Bindable var s = m
        List(selection: Binding(get: { m.search.isEmpty ? m.page : nil }, set: { if let v = $0 { m.page = v; m.search = ""; m.scrollTo = nil; activeGroup = nil } })) {
            profileRow.tag("Profile")
            if !m.changed.isEmpty || m.page == "Changed", let n = m.catalog.navItems["Changed"] { navRow(n, badge: m.changed.count).tag("Changed") }
            ForEach(Array(m.catalog.navGroups.enumerated()), id: \.offset) { _, grp in
                Section {
                    ForEach(grp.1, id: \.self) { id in
                        if let n = m.catalog.navItems[id] {
                            navRow(n).tag(id)
                            if m.page == id && m.search.isEmpty, let p = m.catalog.page(id) {
                                ForEach(p.groups.filter { !$0.title.isEmpty }, id: \.title) { g in
                                    let on = (activeGroup ?? p.groups.first { !$0.title.isEmpty }?.title) == g.title
                                    Button { activeGroup = g.title; m.scrollTo = g.title } label: {
                                        Text(g.title).fontWeight(on ? .semibold : .regular).foregroundStyle(on ? .primary : .secondary)
                                    }.buttonStyle(.plain).padding(.leading, 28)
                                }
                            }
                        }
                    }
                } header: { if let h = grp.0 { Text(h) } }
            }
        }.listStyle(.sidebar)
    }

    var profileRow: some View {
        HStack(spacing: Space.s2) {
            Circle().fill(t.primary).frame(width: 28, height: 28).overlay(Text(m.profileInfo.letter).fontWeight(.semibold).foregroundStyle(t.onPrimary))
            VStack(alignment: .leading, spacing: 0) { Text(m.profileInfo.name).fontWeight(.semibold); Text("Profile").role(.caption).foregroundStyle(.secondary) }
            Spacer(minLength: 0)
            Image(systemName: "chevron.up.chevron.down").font(.system(size: 10, weight: .semibold)).foregroundStyle(.secondary)
        }.padding(.vertical, Space.s1)
    }

    func navRow(_ n: NavItem, badge: Int = 0) -> some View {
        Label {
            HStack { Text(m.words(n.id + ".nav", n.title)); Spacer(); if badge > 0 { Chip(text: "\(badge)", fill: t.primary, ink: t.onPrimary) } }
        } icon: {
            RoundedRectangle(cornerRadius: Radius.control).fill(Color(css: n.colour)).frame(width: 20, height: 20)
                .overlay(Image(systemName: n.symbol).font(.system(size: 11, weight: .medium)).foregroundStyle(.white))
        }
    }
}

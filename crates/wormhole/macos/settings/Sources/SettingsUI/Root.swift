import SwiftUI
import AppKit

/// The Settings window's content, over the model the host keeps (to move the page of a window that is already open).
struct SettingsView: View {
    @State var model: SettingsModel

    var body: some View {
        Inner().environment(model).preferredColorScheme(model.colourScheme ?? (model.value("System:appearance") == "light" ? .light : .dark))
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
                ContentView()
                    .toolbar { ToolbarItemGroup(placement: .navigation) { NavigationButtons() } }
                    .toolbarBackground(.hidden, for: .windowToolbar).modifier(NoToolbarTitle())
            }
            .navigationTitle("Settings")
            .searchable(text: $s.search, placement: .sidebar, prompt: m.w("Search"))
            .modifier(SettingsSearchFocus())
            .environment(\.tok, t).tint(t.primary)
        }
    }
}

/// Programmatic search focus is native from macOS 15; macOS 14 keeps its standard searchable field.
private struct SettingsSearchFocus: ViewModifier {
    @Environment(SettingsModel.self) private var m
    @FocusState private var focused: Bool
    func body(content: Content) -> some View {
        if #available(macOS 15.0, *) {
            content.searchFocused($focused).background {
                Button(m.w("Search")) { focused = true }.keyboardShortcut("f", modifiers: .command)
                    .hidden().accessibilityHidden(true)
            }
        } else { content }
    }
}

/// The sidebar: a native source list with the board's nav tiles. A page of several groups opens into its sub-pages:
/// each is a selection of its own (page, group) that the content shows alone.
struct SidebarView: View {
    @Environment(SettingsModel.self) var m
    @Environment(\.tok) var t
    @Environment(\.accessibilityReduceMotion) var reduceMotion

    var body: some View {
        List(selection: Binding(get: { m.isSearching ? nil : m.sel }, set: { if let v = $0 { m.sel = v; m.search = "" } })) {
            profileRow.tag(Sel(page: "Profile"))
            if !m.changed.isEmpty || m.page == "Changed", let n = m.catalog.navItems["Changed"] { navRow(n, badge: m.changed.count).tag(Sel(page: "Changed")) }
            // A page this build doesn't show (Feature flags in a release, a Preview page whose flag is off) is left out.
            ForEach(Array(m.catalog.navGroups.map { ($0.0, $0.1.filter { !m.store.hidden.contains($0) }) }.filter { !$0.1.isEmpty }.enumerated()), id: \.offset) { _, grp in
                Section {
                    ForEach(grp.1, id: \.self) { id in
                        if let n = m.catalog.navItems[id] {
                            if let p = m.catalog.page(id), !p.subpages.isEmpty {
                                DisclosureGroup(isExpanded: open(id)) {
                                    ForEach(p.subpages, id: \.self) { g in subRow(p, g) }
                                } label: { navRow(n).tag(Sel(page: id)) }
                            } else { navRow(n).tag(Sel(page: id)) }
                        }
                    }
                } header: { if let h = grp.0 { Text(m.w(h)) } }
            }
        }.listStyle(.sidebar)
        .animation(Nav.motion(reduceMotion), value: m.expanded)
        .onChange(of: m.page, initial: true) { _, p in if m.expanded != [p] { m.expanded = [p] } }
    }

    /// A page's open state: the selected page is open; collapsing it takes the selection back to the page itself.
    func open(_ id: String) -> Binding<Bool> {
        Binding(get: { m.expanded.contains(id) }, set: { on in
            if on { m.expanded.insert(id) } else { m.expanded.remove(id); if m.page == id { m.sel = Sel(page: id) } }
        })
    }

    /// One sub-page: indented under its page, with a thin guide line on its left that takes the accent when it is the one shown.
    func subRow(_ p: Page, _ g: Int) -> some View {
        let on = m.page == p.id && !m.isSearching && m.shownGroup == g
        let last = g == p.subpages.last
        return HStack(spacing: Nav.indent) {
            Rectangle().fill(on ? t.primary : Color.secondary.opacity(0.25)).frame(width: on ? Nav.guideSelectedWidth : Nav.guideWidth).frame(maxHeight: .infinity)
            Text(m.w(p.groups[g].title)).foregroundStyle(on ? .primary : .secondary)
        }
        .padding(.bottom, last ? Nav.groupGap : 0)
        .tag(Sel(page: p.id, group: g))
        .transition(.opacity)
    }

    var profileRow: some View {
        HStack(spacing: Space.s2) {
            Avatar(letter: m.profileInfo.letter, fill: t.primary, ink: t.onPrimary)
            VStack(alignment: .leading, spacing: 0) { Text(m.profileInfo.name).fontWeight(.semibold); Text(m.w("Profile")).role(.caption).foregroundStyle(.secondary) }
            Spacer(minLength: 0)
        }.padding(.vertical, 2)
    }

    func navRow(_ n: NavItem, badge: Int = 0) -> some View {
        Label {
            HStack { Text(m.w(n.title)); Spacer(); if badge > 0 { Chip(text: "\(badge)", fill: t.primary, ink: t.onPrimary) } }
        } icon: {
            Tile(colour: Color(css: n.colour), symbol: n.symbol)
        }
    }
}

/// The window keeps its title ("Settings", for the Window menu and VoiceOver) but the toolbar does not draw it: the page has its own.
private struct NoToolbarTitle: ViewModifier {
    func body(content: Content) -> some View {
        if #available(macOS 15.0, *) { content.toolbar(removing: .title) } else { content }
    }
}

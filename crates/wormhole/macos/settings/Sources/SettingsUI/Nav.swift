import SwiftUI

/// What the sidebar has selected: a page, or one of its sub-pages (a group, by index). No group means the page's first.
struct Sel: Hashable {
    var page: String
    var group: Int? = nil
}

extension Page {
    /// The groups that are sub-pages in the sidebar: a page of several groups that all have a title. Other pages are one page.
    var subpages: [Int] { groups.count > 1 && groups.allSatisfy { !$0.title.isEmpty } ? Array(groups.indices) : [] }

    /// Whether the page's preview goes with this sub-page: the rows of the groups it names are what the preview draws.
    func previews(_ group: Int) -> Bool {
        !preview.isEmpty && (previewGroups[id].map { $0.contains(groups[group].title) } ?? true)
    }
}

/// The groups whose rows drive their page's preview; a page not listed here shows it on every sub-page.
let previewGroups: [String: Set<String>] = [
    "Terminal": ["Behaviour"],
    "Appearance": ["Theme", "Accent", "Window", "Details", "Title row"],
    "Sessions": ["Layout", "Groups", "Row details"],
    "Notifications": ["Notify me when", "Delivery"],
    "Agents": ["Agents on this Mac"],
    "MenuBar": ["Menu bar"],
]

extension SettingsModel {
    var page: String { get { sel.page } set { sel = Sel(page: newValue) } }

    /// The sub-page shown now: the selected group, else the page's first.
    var shownGroup: Int { sel.group ?? 0 }

    /// Go where devctl or the app says: `Page` or `Page/<group index or title>`.
    func go(_ spec: String) {
        let parts = spec.split(separator: "/", maxSplits: 1).map(String.init)
        guard let id = parts.first, !id.isEmpty else { return }
        var group: Int?
        if parts.count > 1, let p = catalog.page(id) {
            group = Int(parts[1]) ?? p.groups.firstIndex { $0.title.caseInsensitiveCompare(parts[1]) == .orderedSame }
            if let g = group, !p.subpages.contains(g) { group = nil }
        }
        sel = Sel(page: id, group: group)
        search = ""
    }

    func goBack() { guard let s = back.popLast() else { return }; navigating = true; fwd.append(sel); sel = s; search = ""; navigating = false }
    func goForward() { guard let s = fwd.popLast() else { return }; navigating = true; back.append(sel); sel = s; search = ""; navigating = false }
    var canGoBack: Bool { !back.isEmpty }
    var canGoForward: Bool { !fwd.isEmpty }

    /// Called as the selection changes by the person or the app (not by back and forward).
    func record(from old: Sel) {
        guard !navigating, old != sel else { return }
        back.append(old); fwd = []
    }
}

/// Native toolbar buttons also expose the standard history shortcuts.
struct NavigationButtons: View {
    @Environment(SettingsModel.self) var m
    var body: some View {
        Button { m.goBack() } label: { Label(m.w("Back"), systemImage: "chevron.backward") }
            .keyboardShortcut("[", modifiers: .command).disabled(!m.canGoBack).help(m.w("Back"))
        Button { m.goForward() } label: { Label(m.w("Forward"), systemImage: "chevron.forward") }
            .keyboardShortcut("]", modifiers: .command).disabled(!m.canGoForward).help(m.w("Forward"))
    }
}

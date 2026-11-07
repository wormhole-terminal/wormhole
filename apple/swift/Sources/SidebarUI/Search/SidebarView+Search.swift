// The search over the list. The core filters (`Action::Search`: every word, any case, over a terminal's name, folder,
// branch and words; a group with nothing found is not listed, a new terminal ends the search); the view only shows the
// field, keeps its text in step with the core's and answers the keys. The field is there while a search is on or was
// asked for with ⌘F; Esc is the ladder's nearest rung (the field, then the window's own): it ends the search and gives
// the keyboard back to the list.

import DesignSystem
import SwiftUI
import WorkspaceModel

extension SidebarView {
    var searching: Bool { !state.search.isEmpty }
    private var shown: Bool { searchOpen || searching }

    @ViewBuilder var searchBar: some View {
        if shown {
            let margin = SidebarMetrics(density: appearance.density, cards: appearance.cards).cardMargin
            SidebarSearchField(
                text: $searchText, found: state.terminals.count + foldedMatches, total: state.all.count, focus: $searchFocused,
                exit: endSearch, submit: openFirst, toList: toList
            )
            .padding(.horizontal, margin).padding(.top, Space._1)
            .transition(.opacity.combined(with: .move(edge: .top)))
        }
    }

    /// Terminals that match but stand folded away in their group: the head's tiles carry them.
    private var foldedMatches: Int { state.sections.filter { $0.head?.isFolded == true }.reduce(0) { $0 + $1.terminalCount } }

    func revealSearch() {
        withAnimation(reduce ? Motion.fade : SidebarMotion.fold) { searchOpen = true }
        // The field joins the view in this pass; the keyboard goes to it in the next.
        Task { @MainActor in searchFocused = true }
    }

    /// Esc: the search ends and the list has the keyboard again, on the terminal that was on screen.
    func endSearch() {
        searchText = ""
        send(.search(""))
        withAnimation(reduce ? Motion.fade : SidebarMotion.fold) { searchOpen = false }
        toList()
    }

    /// Return: the first terminal found opens, and the list keeps the search for ↑ ↓.
    private func openFirst() {
        guard let first = state.terminals.first else { return }
        send(.select(first.id))
        toList()
    }

    func toList() {
        searchFocused = false
        focused = true
        cursor = state.selected.map(SidebarStop.row) ?? SidebarWalk(state.sections).items.first?.stop
        keysLead = true
    }
}

/// What the list says when the search finds nothing.
struct NothingFound: View {
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(SidebarText.searchNothing).font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textMuted)
            .padding(.top, Space._6)
            .allowsHitTesting(false)
            .accessibilityAddTraits(.isStaticText)
    }
}

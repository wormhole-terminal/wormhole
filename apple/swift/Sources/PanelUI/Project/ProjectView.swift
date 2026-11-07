// The right panel's Project tab (`features/project/panel.rs`): the colour of the active terminal's folder, then its
// `.wormhole/` folder and its trust, bands for the per-feature switches, the actions, the prompts and the terminal layout,
// with two-line rows like the sidebar's; or, in a folder with no `.wormhole/`, the button that makes one.

import DesignSystem
import PanelModel
import SwiftUI

public struct ProjectView: View {
    @Bindable var store: ProjectStore
    let notes: NotesStore
    @Environment(\.tokens) private var tokens

    public init(store: ProjectStore, notes: NotesStore) {
        self.store = store
        self.notes = notes
    }

    public var body: some View {
        PanelScroll {
            if store.folder != nil { FolderColorCard(store: store) }
            if let state = store.state {
                ProjectHeadCard(store: store, state: state)
                ProjectFeaturesCard(store: store, state: state)
                if state.actions.isEmpty == false, enabled(.actions, state) { ProjectActionsCard(store: store, actions: state.actions) }
                if state.prompts.isEmpty == false, enabled(.prompts, state) { ProjectPromptsCard(store: store, notes: notes, prompts: state.prompts) }
                if !state.layout.isEmpty, enabled(.layout, state) { ProjectLayoutCard(store: store, names: state.layout) }
            } else if store.loaded {
                NoProjectCard(store: store)
            }
        }
        .sheet(isPresented: $store.showingAllLines) {
            if let state = store.state { TrustLinesSheet(state: state) { store.showingAllLines = false } }
        }
    }

    private func enabled(_ feature: ProjectFeature, _ state: ProjectState) -> Bool {
        state.features.first { $0.feature == feature }?.on ?? false
    }
}

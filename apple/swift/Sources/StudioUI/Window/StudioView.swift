// The Theme Studio window's content: the title row, the notice, and three columns, the token list on the left,
// the gallery of every component on the live draft in the middle, the token editor on the right
// (`studio/shell.rs`). It is themed by the store like every window, and the store shows the draft, so the whole
// window (and every other) follows each edit.

import DesignSystem
import StudioModel
import SwiftUI

public struct StudioView: View {
    @Bindable var session: StudioSession
    let store: ThemeStore
    @State private var showsVersions = false
    @Environment(\.tokens) private var tokens

    public init(session: StudioSession, store: ThemeStore = .shared) {
        self.session = session
        self.store = store
    }

    public var body: some View {
        StudioContent(session: session, showsVersions: $showsVersions)
            .environment(\.textScale, store.textScale)
            .themed(store)
    }
}

private struct StudioContent: View {
    @Bindable var session: StudioSession
    @Binding var showsVersions: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        let document = session.document
        VStack(spacing: 0) {
            TitleBar(session: session, showsVersions: $showsVersions)
            NoticeRow(session: session)
            HStack(spacing: 0) {
                TokenListView(document: document)
                GalleryView(document: document)
                TokenEditorView(document: document)
            }
        }
        .foregroundStyle(tokens.textPrimary)
        .background(tokens.surfaceWindow)
        .animation(Motion.layout, value: document.notice)
        .sheet(isPresented: $showsVersions) { VersionsSheet(document: document) { showsVersions = false }.themed() }
        .sheet(isPresented: $session.showsUpdate) {
            UpdateSheet(check: document.update, apply: { session.applyUpdate() }, cancel: { session.showsUpdate = false }).themed()
        }
        // Esc ends a rename first, and then closes the window (asking about unsaved edits as the red light does).
        .onExitCommand { if session.renaming { session.renaming = false } else { session.window?.performClose(nil) } }
        .onChange(of: ThemeStore.shared.shownPick) { document.refreshInUse() }
    }
}

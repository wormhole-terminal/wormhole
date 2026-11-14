// The Theme Studio window's content: the title row, the notice, and three columns, the token list on the left,
// the gallery of every component on the live draft in the middle, the token editor on the right
// (`studio/shell.rs`). It is themed by the store like every window, and the store shows the draft, so the whole
// window (and every other) follows each edit. `Extras` are the gallery's specimens of the window's own module, built by
// id by whoever opens the Studio (the app); without them (a test, a preview) those specimens are empty.

import DesignSystem
import StudioModel
import SwiftUI

public struct StudioView<Extras: View>: View {
    @Bindable var session: StudioSession
    let store: ThemeStore
    let extras: @MainActor (String) -> Extras
    @State private var showsVersions = false
    @Environment(\.tokens) private var tokens

    public init(session: StudioSession, store: ThemeStore = .shared, @ViewBuilder extras: @escaping @MainActor (String) -> Extras) {
        self.session = session
        self.store = store
        self.extras = extras
    }

    public var body: some View {
        StudioContent(session: session, showsVersions: $showsVersions, extras: extras)
            .environment(\.textScale, store.textScale)
            .themed(store)
    }
}

extension StudioView where Extras == EmptyView {
    /// The Studio without the window's specimens: for a test or a preview, which has no window module.
    public init(session: StudioSession, store: ThemeStore = .shared) {
        self.init(session: session, store: store) { _ in EmptyView() }
    }
}

private struct StudioContent<Extras: View>: View {
    @Bindable var session: StudioSession
    @Binding var showsVersions: Bool
    let extras: @MainActor (String) -> Extras
    @Environment(\.tokens) private var tokens

    var body: some View {
        let document = session.document
        VStack(spacing: 0) {
            TitleBar(session: session, showsVersions: $showsVersions)
            NoticeRow(session: session)
            HStack(spacing: 0) {
                TokenListView(document: document)
                GalleryView(document: document, extras: extras)
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
        // Increase Contrast turned on or off in System Settings: the draft is resolved again for it, and its contrast report
        // (the list's marks, the Low filter, the Contrast section, the card) is asked again with it (`tokens_of`).
        .onChange(of: document.increaseContrast) { document.refresh() }
    }
}

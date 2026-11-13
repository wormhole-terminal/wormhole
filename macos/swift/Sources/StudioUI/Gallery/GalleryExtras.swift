// The specimens of modules above the Studio: the window's own chrome (its status bar, the right panel's icon tabs, a
// column's resize handle), which this module may not name. The app sets the source when it opens the Studio; until then
// (a preview, a test without the window) those specimens are empty.

import SwiftUI

@MainActor
public enum GalleryExtras {
    /// The view of the specimen `id` names, or nil where nobody draws it.
    public static var view: @MainActor (_ id: String) -> AnyView? = { _ in nil }
}

/// A specimen another module draws.
struct ExtraSpecimen: View {
    let id: String

    var body: some View {
        if let view = GalleryExtras.view(id) { view.frame(maxWidth: .infinity, alignment: .leading) }
    }
}

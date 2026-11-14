// Opens the project commands window (the trust notice's "View all N…", the Project tab's button).

import AuxModel
import SwiftUI

@MainActor
public enum TrustWindow {
    private static let size = CGSize(width: 640, height: 480)
    private static let minSize = CGSize(width: 480, height: 320)

    /// The project the open window shows, so a second project's commands replace the first's instead of the window staying on it.
    private static var shownRoot: String?
    private static weak var model: TrustViewModel?

    /// What the open window draws (its model), for the control folder's `trust-state`; nil while it is closed.
    public static var shown: TrustViewModel? { AuxWindowController.current("trust") == nil ? nil : model }

    public static func show(backend: any TrustBackend, root: String? = nil) {
        let other = AuxWindowController.current("trust") != nil && shownRoot != root
        shownRoot = root
        AuxWindowController.show(key: "trust", title: AuxText.trustWindowTitle, size: size, minSize: minSize, rebuilds: other) {
            page(backend)
        }
    }

    private static func page(_ backend: any TrustBackend) -> some View {
        let model = TrustViewModel(backend: backend)
        Self.model = model
        return TrustView(model: model)
    }
}

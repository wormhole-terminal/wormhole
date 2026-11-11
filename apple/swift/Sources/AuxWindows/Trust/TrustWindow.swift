// Opens the project commands window (the trust notice's "View all N…", the Project tab's button).

import AuxModel
import SwiftUI

@MainActor
public enum TrustWindow {
    private static let size = CGSize(width: 640, height: 480)
    private static let minSize = CGSize(width: 480, height: 320)

    /// The project the open window shows, so a second project's commands replace the first's instead of the window staying on it.
    private static var shownRoot: String?

    public static func show(backend: any TrustBackend, root: String? = nil) {
        let other = AuxWindowController.current("trust") != nil && shownRoot != root
        shownRoot = root
        AuxWindowController.show(key: "trust", title: AuxText.trustWindowTitle, size: size, minSize: minSize, rebuilds: other) {
            TrustView(model: TrustViewModel(backend: backend))
        }
    }
}

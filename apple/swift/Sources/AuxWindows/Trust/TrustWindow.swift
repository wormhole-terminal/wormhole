// Opens the project commands window (the trust notice's "View all N…", the Project tab's button).

import AuxModel
import SwiftUI

@MainActor
public enum TrustWindow {
    private static let size = CGSize(width: 640, height: 480)
    private static let minSize = CGSize(width: 480, height: 320)

    public static func show(backend: any TrustBackend) {
        AuxWindowController.show(key: "trust", title: AuxText.trustWindowTitle, size: size, minSize: minSize) {
            TrustView(model: TrustViewModel(backend: backend))
        }
    }
}

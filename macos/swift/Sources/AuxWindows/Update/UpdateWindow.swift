// Opens the Updates window (Wormhole > Check for Updates…, and the notice's click when an update is ready).

import AuxModel
import SwiftUI

@MainActor
public enum UpdateWindow {
    private static let size = CGSize(width: 520, height: 560)
    private static let minSize = CGSize(width: 440, height: 400)

    public static func show(backend: any UpdateBackend, version: String) {
        var controller: AuxWindowController?
        controller = AuxWindowController.show(key: "updates", title: AuxText.updatesTitle, size: size, minSize: minSize) {
            UpdateView(model: UpdateViewModel(backend: backend), version: version, sheetWindow: { controller?.window })
        }
    }
}

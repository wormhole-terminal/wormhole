// Opens the Session Hosts window.

import AppKit
import AuxModel
import DesignSystem
import SwiftUI

@MainActor
public enum HostsWindow {
    private static let size = CGSize(width: 940, height: 720)
    private static let minSize = CGSize(width: 640, height: 420)

    /// The open window's model, for the control folder's `hosts-toggle` and `hosts-scroll` (nil while it is closed).
    public private(set) static weak var model: HostsViewModel?

    public static func show(backend: any HostsBackend) {
        var controller: AuxWindowController?
        controller = AuxWindowController.show(key: "hosts", title: AuxText.hostsTitle, size: size, minSize: minSize) {
            page(backend: backend, sheetWindow: { controller?.window })
        }
    }

    private static func page(backend: any HostsBackend, sheetWindow: @escaping () -> NSWindow?) -> some View {
        let model = HostsViewModel(backend: backend) { text in
            Pasteboard.setText(text)
        }
        Self.model = model
        return HostsView(model: model, sheetWindow: sheetWindow).secondaryBackdrop(.hosts)
    }
}

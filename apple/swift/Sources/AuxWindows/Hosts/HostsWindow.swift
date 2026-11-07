// Opens the Session Hosts window.

import AppKit
import AuxModel
import SwiftUI

@MainActor
public enum HostsWindow {
    private static let size = CGSize(width: 940, height: 720)
    private static let minSize = CGSize(width: 640, height: 420)

    public static func show(backend: any HostsBackend) {
        var controller: AuxWindowController?
        controller = AuxWindowController.show(key: "hosts", title: AuxText.hostsTitle, size: size, minSize: minSize) {
            HostsView(
                model: HostsViewModel(backend: backend) { text in
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(text, forType: .string)
                },
                sheetWindow: { controller?.window })
        }
    }
}

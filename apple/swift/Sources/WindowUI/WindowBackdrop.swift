// What lies behind every surface: the system's blur material when the window is translucent (the GPUI app
// puts an `NSVisualEffectView` of the HUD window material there, `platform/macos/vibrancy.rs`), a plain fill of
// the window's colour when it is opaque (Reduce Transparency, Increase Contrast, opacity 1). Surfaces paint
// translucent over it, so the blur is what shows through.

import AppKit
import SwiftUI

struct WindowBackdrop: View {
    @Environment(WindowLook.self) private var look
    @Environment(\.chrome) private var colours

    var body: some View {
        ZStack {
            // Under everything and always there: a translucent window with the blur off shows the desktop.
            if !look.isTranslucent { colours.terminal }
            if look.blurShown { BlurMaterial() }
        }
        .ignoresSafeArea()
        .accessibilityHidden(true)
    }
}

private struct BlurMaterial: NSViewRepresentable {
    func makeNSView(context: Context) -> NSVisualEffectView {
        let view = NSVisualEffectView()
        view.material = .hudWindow
        view.blendingMode = .behindWindow
        // Active even in a window that is not key, as the GPUI app's is: the chrome must not change colour
        // when focus moves to another app.
        view.state = .active
        return view
    }

    func updateNSView(_ view: NSVisualEffectView, context: Context) {}
}

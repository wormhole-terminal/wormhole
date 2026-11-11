// The window verbs of the Window menu and the title bar: minimise, zoom, full screen and Bring All to Front, on
// the window that is key. AppKit does them; this names them once, and says whether each can run now, which the
// menu's checked and disabled state needs (`menu_state`'s full-screen flip).

import AppKit

@MainActor
public enum WindowCommands {
    public static func minimize(_ window: NSWindow? = NSApp.keyWindow) { window?.performMiniaturize(nil) }
    public static func zoom(_ window: NSWindow? = NSApp.keyWindow) { window?.performZoom(nil) }
    public static func toggleFullScreen(_ window: NSWindow? = NSApp.keyWindow) { window?.toggleFullScreen(nil) }
    public static func bringAllToFront() { NSApp.arrangeInFront(nil) }

    public static func isFullScreen(_ window: NSWindow? = NSApp.keyWindow) -> Bool {
        window?.styleMask.contains(.fullScreen) ?? false
    }

    /// The traffic lights shown or taken out of sight and out of reach (the intro window keeps them away).
    public static func setTrafficLights(shown: Bool, in window: NSWindow) {
        for kind in [NSWindow.ButtonType.closeButton, .miniaturizeButton, .zoomButton] {
            window.standardWindowButton(kind)?.isHidden = !shown
        }
    }
}

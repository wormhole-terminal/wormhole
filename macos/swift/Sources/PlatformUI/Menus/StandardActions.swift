// The actions AppKit answers itself, so they work with any first responder and the system's own behaviour
// (hide, minimise, full screen, the Services key, a text field's editing keys). Everything else in the keymap
// reaches the app through the `MenuDispatcher`.

import AppKit
import PlatformModel

@MainActor
public enum StandardActions {
    public static let selectors: [MenuCommand: Selector] = [
        .action("hide"): #selector(NSApplication.hide(_:)),
        .action("hideOthers"): #selector(NSApplication.hideOtherApplications(_:)),
        .action("showAll"): #selector(NSApplication.unhideAllApplications(_:)),
        .action("quit"): #selector(NSApplication.terminate(_:)),
        .action("minimize"): #selector(NSWindow.performMiniaturize(_:)),
        .action("zoom"): #selector(NSWindow.performZoom(_:)),
        .action("bringAllToFront"): #selector(NSApplication.arrangeInFront(_:)),
        .action("fullScreen"): #selector(NSWindow.toggleFullScreen(_:)),
        // A text field's editing keys, and the terminal's where it answers the same selectors.
        .action("undo"): Selector(("undo:")),
        .action("redo"): Selector(("redo:")),
        .action("cut"): #selector(NSText.cut(_:)),
        .action("copy"): #selector(NSText.copy(_:)),
        .action("paste"): #selector(NSText.paste(_:)),
        .action("selectAll"): #selector(NSText.selectAll(_:)),
    ]
}

// The words of the panes' views, through one seam: each is a localisable string whose key is the Fluent
// message's name in `wormhole-text` (`locales/en/*.ftl`), the English being the default until the catalogs
// come through the FFI. No view writes a sentence of its own.

import Foundation
import PanesModel

enum PanesText {
    static var splitRight: String { String(localized: "pane-menu-split-right", defaultValue: "Split right") }
    static var splitDown: String { String(localized: "pane-menu-split-down", defaultValue: "Split down") }
    static var zoom: String { String(localized: "pane-menu-zoom", defaultValue: "Zoom pane") }
    static var unzoom: String { String(localized: "pane-menu-unzoom", defaultValue: "Unzoom pane") }
    static var leaveSplit: String { String(localized: "pane-menu-leave-split", defaultValue: "Move out of split") }
    static var closePane: String { String(localized: "pane-close", defaultValue: "Close pane") }
    static var paneMenu: String { String(localized: "pane-menu", defaultValue: "Pane menu") }
    static var divider: String { String(localized: "pane-divider", defaultValue: "Pane divider") }
    static var dividerHint: String {
        String(localized: "pane-divider-hint", defaultValue: "Drag to resize · double-click to equalize")
    }

    /// "claude, ~/Projects/tube, pane 1 of 3, focused": a header's VoiceOver label (`spoken::pane`).
    static func paneLabel(_ name: String, folder: String?, position: Int, of: Int, focused: Bool) -> String {
        let place = String(localized: "pane-spoken-position", defaultValue: "pane \(position) of \(of)")
        let state = String(localized: "pane-spoken-focused", defaultValue: "focused")
        return [name, folder.flatMap { $0.isEmpty ? nil : $0 }, place, focused ? state : nil].compactMap { $0 }.joined(separator: ", ")
    }

    /// What a hint on a zone says a drop will do.
    static func dropHint(_ zone: Zone, moving: Bool, target: String) -> String {
        switch (moving, zone) {
        case (false, .left): String(localized: "pane-drop-show-left", defaultValue: "Show on the left")
        case (false, .right): String(localized: "pane-drop-show-right", defaultValue: "Show on the right")
        case (false, .top): String(localized: "pane-drop-show-above", defaultValue: "Show above")
        case (false, .bottom): String(localized: "pane-drop-show-below", defaultValue: "Show below")
        case (true, .left): String(localized: "pane-drop-place-left", defaultValue: "Place left of \(target)")
        case (true, .right): String(localized: "pane-drop-place-right", defaultValue: "Place right of \(target)")
        case (true, .top): String(localized: "pane-drop-place-above", defaultValue: "Place above \(target)")
        case (true, .bottom): String(localized: "pane-drop-place-below", defaultValue: "Place below \(target)")
        }
    }
}

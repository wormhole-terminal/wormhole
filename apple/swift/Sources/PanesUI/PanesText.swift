// The words of the panes' views, through one seam: each is a localisable string whose key is the Fluent
// message's name in `wormhole-text` (`locales/en/*.ftl`), the English being the default until the catalogs
// come through the FFI. No view writes a sentence of its own.

import Foundation
import PanesModel
import WordsModel

enum PanesText {
    static var splitRight: String { Words.text("pane-menu-split-right", fallback: "Split right") }
    static var splitDown: String { Words.text("pane-menu-split-down", fallback: "Split down") }
    static var zoom: String { Words.text("pane-menu-zoom", fallback: "Zoom pane") }
    static var unzoom: String { Words.text("pane-menu-unzoom", fallback: "Unzoom pane") }
    static var leaveSplit: String { Words.text("pane-menu-leave-split", fallback: "Move out of split") }
    static var closePane: String { Words.text("pane-close", fallback: "Close pane") }
    static var paneMenu: String { Words.text("pane-menu", fallback: "Pane menu") }
    static var divider: String { Words.text("pane-divider", fallback: "Pane divider") }
    static var dividerHint: String {
        Words.text("pane-divider-hint", fallback: "Drag to resize · double-click to equalize")
    }

    /// "claude, ~/Projects/tube, pane 1 of 3, focused": a header's VoiceOver label (`spoken::pane`).
    static func paneLabel(_ name: String, folder: String?, position: Int, of: Int, focused: Bool) -> String {
        let place = Words.text("pane-spoken-position", fallback: "pane \(position) of \(of)")
        let state = Words.text("pane-spoken-focused", fallback: "focused")
        return [name, folder.flatMap { $0.isEmpty ? nil : $0 }, place, focused ? state : nil].compactMap { $0 }.joined(separator: ", ")
    }

    /// What a hint on a zone says a drop will do.
    static func dropHint(_ zone: Zone, moving: Bool, target: String) -> String {
        switch (moving, zone) {
        case (false, .left): Words.text("pane-drop-show-left", fallback: "Show on the left")
        case (false, .right): Words.text("pane-drop-show-right", fallback: "Show on the right")
        case (false, .top): Words.text("pane-drop-show-above", fallback: "Show above")
        case (false, .bottom): Words.text("pane-drop-show-below", fallback: "Show below")
        case (true, .left): Words.text("pane-drop-place-left", fallback: "Place left of \(target)")
        case (true, .right): Words.text("pane-drop-place-right", fallback: "Place right of \(target)")
        case (true, .top): Words.text("pane-drop-place-above", fallback: "Place above \(target)")
        case (true, .bottom): Words.text("pane-drop-place-below", fallback: "Place below \(target)")
        }
    }
}

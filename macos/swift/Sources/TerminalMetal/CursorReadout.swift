// text-file: the control folder's `cursor` read-out (dev tooling), not interface text
// The cursor as the next frame draws it: `drawn=` is what the scene hands its row's build (`TerminalScene.cursorDraw`: none
// once the program ended or in a blinking cursor's off phase) and the face the builder paints for it (`CursorDraw.face`), the
// same two values, not a copy of their rules. The rest are the facts they are made from: the engine's cursor (place, DECSCUSR
// shape and blink), the blink phase, whether this view has the keys, and the palette slot it is painted in (the theme's,
// with a program's OSC 12 over it).
//
//     cursor drawn=outline shape=block blinking=false on=true focused=false hidden=false at=0,4 colour=#ff8800

import Foundation
import TerminalCore

extension MetalRenderer {
    var cursorReadout: String {
        guard let cursor = scene.cursor else { return "cursor none" }
        let drawn = scene.cursorDraw.map { "\($0.face)" } ?? "none"
        let colour = palette.cursor
        return "cursor drawn=\(drawn) shape=\(cursor.shape) blinking=\(cursor.blinking) on=\(scene.cursorOn) focused=\(scene.focused) "
            + "hidden=\(scene.cursorHidden) at=\(cursor.row),\(cursor.column) colour=" + String(format: "#%02x%02x%02x", Int(colour.x), Int(colour.y), Int(colour.z))
    }
}

extension MetalTerminalView {
    public var cursorReadout: String { renderer.cursorReadout }
}

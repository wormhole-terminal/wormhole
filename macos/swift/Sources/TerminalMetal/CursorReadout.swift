// text-file: the control folder's `cursor` read-out (dev tooling), not interface text
// The cursor as the next frame draws it, from exactly what `TerminalScene` builds its row from: the engine's cursor (place,
// DECSCUSR shape and blink), the blink phase, whether the program has ended, whether this view has the keys, and the palette
// slot it is painted in (the theme's, with a program's OSC 12 over it). Nothing here decides.
//
//     cursor drawn=outline shape=block blinking=false on=true focused=false hidden=false at=0,4 colour=#ff8800

import Foundation
import TerminalCore

extension MetalTerminalView {
    public var cursorReadout: String {
        let scene = renderer.scene
        guard let cursor = scene.cursor else { return "cursor none" }
        // `TerminalScene`'s row build: no cursor once the program ended, or in a blinking cursor's off phase.
        let shown = !scene.cursorHidden && (scene.cursorOn || !cursor.blinking)
        // `RowBuilder`: a filled block, a beam or an underline while focused; the cell's outline when not.
        let drawn = !shown ? "none" : scene.focused ? "\(cursor.shape)" : "outline"
        let colour = renderer.palette.cursor
        return "cursor drawn=\(drawn) shape=\(cursor.shape) blinking=\(cursor.blinking) on=\(scene.cursorOn) focused=\(scene.focused) "
            + "hidden=\(scene.cursorHidden) at=\(cursor.row),\(cursor.column) colour=" + String(format: "#%02x%02x%02x", Int(colour.x), Int(colour.y), Int(colour.z))
    }
}

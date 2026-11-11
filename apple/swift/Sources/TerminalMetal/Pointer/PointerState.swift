// What the view remembers between pointer events.

import AppKit
import TerminalCore

struct PointerState {
    /// A button the program has: its release and drags go to it, not to a selection.
    var reporting: PointerButton?
    /// A selection is being dragged.
    var selecting = false
    /// The link under the pointer, while Command is held.
    var link: LinkInfo?
    /// Where the pointer last was, in the view's coordinates: Command going down or up re-asks what is under it,
    /// and a drag outside the grid keeps scrolling from it while the mouse is still.
    var lastPoint: CGPoint?
    /// Whether a program had the mouse when the shape of the pointer was last chosen.
    var programTakes = false
    var autoscroll: DispatchWorkItem?
}

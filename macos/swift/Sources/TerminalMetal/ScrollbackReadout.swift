// text-file: the control folder's `scrollback` read-out (dev tooling), not interface text
// The history as the view was last given it, for the control folder's `scrollback`: the numbers of the last frame's header,
// which the scroll bar is drawn from (`updateScroller`), not a count kept beside them.

extension MetalTerminalView {
    public var scrollbackReadout: String {
        guard let header = renderer.scene.header else { return "scrollback none" }
        return "scrollback history=\(header.historySize) offset=\(header.displayOffset) rows=\(header.rows)"
    }
}

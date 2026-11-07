// The mouse: a press either belongs to a program that asked for it (mouse modes, the locator), opens a link
// (Command), or starts a selection (a click is cells, two words, three lines, Option makes a rectangle).
// Who gets the pointer and what each encodes to are the seam's (`TerminalInteraction`); this turns NSEvents
// into calls. Behaviour after `terminal_view/input.rs`.

import AppKit
import TerminalCore

extension MetalTerminalView {
    /// The grid as the last frame drew it.
    var cellGrid: CellGrid {
        let cell = renderer.cellPixels, scale = CGFloat(renderer.scale), size = renderer.gridSize
        return CellGrid(cellWidth: CGFloat(cell.width) / scale, cellHeight: CGFloat(cell.height) / scale, columns: size.cols, rows: size.rows)
    }

    private func location(_ event: NSEvent) -> CGPoint { convert(event.locationInWindow, from: nil) }

    private func modifiers(_ event: NSEvent) -> PointerModifiers {
        let flags = event.modifierFlags
        return PointerModifiers(shift: flags.contains(.shift), alt: flags.contains(.option), control: flags.contains(.control))
    }

    // MARK: pressing

    public override func mouseDown(with event: NSEvent) {
        // Control-click is a right click on the Mac.
        if event.modifierFlags.contains(.control), openContextMenu(for: event) { return }
        press(.left, event)
    }

    public override func rightMouseDown(with event: NSEvent) {
        if !openContextMenu(for: event) { press(.right, event) }
    }

    public override func otherMouseDown(with event: NSEvent) { press(.middle, event) }

    private func press(_ button: PointerButton, _ event: NSEvent) {
        window?.makeFirstResponder(self)
        endComposition()
        let point = location(event)
        pointer.lastPoint = point
        guard let interaction, let hit = cellGrid.hit(point) else { return }
        if button == .left, event.modifierFlags.contains(.command), openLink(at: hit) { return }
        if send(button, .press, hit, event) {
            pointer.reporting = button
            return
        }
        guard button == .left else { return }
        let kind: SelectionKind =
            switch event.clickCount {
            case 2: .word
            case 3: .line
            default: event.modifierFlags.contains(.option) ? .block : .cell
            }
        interaction.selectionStart(kind: kind, row: hit.row, column: hit.column, side: hit.side)
        pointer.selecting = true
        selectionMayExist = true
        requestRedraw()
    }

    // MARK: dragging and releasing

    public override func mouseDragged(with event: NSEvent) { drag(.left, event) }
    public override func rightMouseDragged(with event: NSEvent) { drag(.right, event) }
    public override func otherMouseDragged(with event: NSEvent) { drag(.middle, event) }

    private func drag(_ button: PointerButton, _ event: NSEvent) {
        let point = location(event)
        pointer.lastPoint = point
        if pointer.reporting != nil {
            if let hit = cellGrid.hit(point) { _ = send(button, .motion, hit, event) }
        } else if pointer.selecting {
            extendSelection(to: point)
        }
    }

    public override func mouseUp(with event: NSEvent) { release(.left, event) }
    public override func rightMouseUp(with event: NSEvent) { release(.right, event) }
    public override func otherMouseUp(with event: NSEvent) { release(.middle, event) }

    private func release(_ button: PointerButton, _ event: NSEvent) {
        if let held = pointer.reporting, held == button {
            pointer.reporting = nil
            if let hit = cellGrid.hit(location(event)) { _ = send(button, .release, hit, event) }
            return
        }
        guard pointer.selecting, button == .left else { return }
        pointer.selecting = false
        pointer.autoscroll?.cancel()
        pointer.autoscroll = nil
        interaction?.selectionFinish()
        requestRedraw()
    }

    // MARK: moving

    public override func mouseMoved(with event: NSEvent) {
        let point = location(event)
        pointer.lastPoint = point
        revealScrollerNear(point)
        guard let hit = cellGrid.hit(point), cellGrid.contains(point) else { return }
        updateLink(under: hit, command: event.modifierFlags.contains(.command))
        _ = send(.none, .motion, hit, event)
    }

    public override func mouseExited(with event: NSEvent) {
        pointer.lastPoint = nil
        updateLink(under: nil, command: false)
    }

    public override func flagsChanged(with event: NSEvent) {
        if let point = pointer.lastPoint, let hit = cellGrid.hit(point), cellGrid.contains(point) {
            updateLink(under: hit, command: event.modifierFlags.contains(.command))
        }
        super.flagsChanged(with: event)
    }

    public override func updateTrackingAreas() {
        super.updateTrackingAreas()
        for area in trackingAreas { removeTrackingArea(area) }
        addTrackingArea(
            NSTrackingArea(
                rect: .zero, options: [.mouseMoved, .mouseEnteredAndExited, .activeInKeyWindow, .inVisibleRect], owner: self, userInfo: nil))
    }

    // MARK: helpers

    /// Give an event to the program; `false` when it did not ask for it or Shift took the pointer back.
    func send(_ button: PointerButton, _ action: PointerAction, _ hit: CellHit, _ event: NSEvent) -> Bool {
        interaction?.reportPointer(
            PointerInput(button: button, action: action, row: hit.row, column: hit.column, x: hit.x, y: hit.y, modifiers: modifiers(event))) ?? false
    }
}

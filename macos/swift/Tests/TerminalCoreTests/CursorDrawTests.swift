import Foundation
import Testing
import TerminalCore

@testable import TerminalMetal

/// The `cursor` read-out names what the row's build is handed and paints (`TerminalScene.cursorDraw`, `CursorDraw.face`),
/// not a second copy of those rules: GPUI's `cursor_rect` (`terminal_view/render/cursor.rs:71`, no cursor after exit `:64`).
@MainActor
@Suite struct CursorDrawTests {
    private func renderer(_ cursor: FrameHeader.Cursor) throws -> MetalRenderer {
        let renderer = try RowDrawing.renderer()
        try renderer.apply(frame: RowDrawing.frame(RowDrawing.blank(), cursor: cursor))
        return renderer
    }

    @Test func theFaceIsTheShapeWithTheKeysAndTheOutlineWithout() {
        for (shape, face) in [(CursorShape.block, CursorDraw.Face.block), (.beam, .beam), (.underline, .underline), (.hollow, .outline)] {
            #expect(CursorDraw(column: 0, shape: shape, wide: false, focused: true).face == face)
            #expect(CursorDraw(column: 0, shape: shape, wide: false, focused: false).face == .outline)
        }
    }

    @Test func theReadoutFollowsWhatTheSceneHandsItsRow() throws {
        let renderer = try renderer(FrameHeader.Cursor(row: 1, column: 2, shape: .beam, blinking: true))
        #expect(renderer.scene.cursorDraw?.face == .beam)
        #expect(renderer.cursorReadout.contains("drawn=beam shape=beam"))

        renderer.setFocused(false)
        #expect(renderer.scene.cursorDraw?.face == .outline)
        #expect(renderer.cursorReadout.contains("drawn=outline shape=beam"))

        // A blinking cursor's off phase, and a program that ended: the row is built without one.
        renderer.setFocused(true)
        renderer.setCursorBlinkOn(false)
        #expect(renderer.scene.cursorDraw == nil)
        #expect(renderer.cursorReadout.contains("drawn=none"))
        renderer.setCursorBlinkOn(true)
        renderer.setCursorHidden(true)
        #expect(renderer.scene.cursorDraw == nil)
        #expect(renderer.cursorReadout.contains("drawn=none") && renderer.cursorReadout.contains("hidden=true"))
    }

    @Test func aScreenWithoutACursorSaysSo() throws {
        let renderer = try RowDrawing.renderer()
        try renderer.apply(frame: RowDrawing.frame(RowDrawing.blank()))
        #expect(renderer.cursorReadout == "cursor none")
    }
}

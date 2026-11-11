import CoreGraphics
import Testing
import WorkspaceModel

@MainActor
@Suite struct TerminalDragPointerTests {
    @Test func theWindowAsksFirstWhetherItTakesATerminalLetGoOutsideIt() {
        let pointer = TerminalDragPointer()
        var asked: [(TerminalID, CGPoint)] = []
        pointer.tearOff = { id, point in
            asked.append((id, point))
            return point.x < 0
        }
        #expect(pointer.tearOff?(4, CGPoint(x: -20, y: 5)) == true)
        #expect(pointer.tearOff?(4, CGPoint(x: 20, y: 5)) == false)
        #expect(asked.count == 2 && asked[0].0 == 4)
        // Not taken: the release is the panes' to answer, as before.
        pointer.release(4, at: CGPoint(x: 20, y: 5))
        #expect(pointer.released == .init(terminal: 4, point: CGPoint(x: 20, y: 5)))
    }
}

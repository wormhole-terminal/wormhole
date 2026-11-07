import Testing
import WindowsModel
import WorkspaceModel

@MainActor
@Suite struct MemoryWindowsTests {
    private let workspace = MockWorkspaceBackend()

    @Test func windowIdsNeverRepeatAndAPlanTakesOne() {
        let windows = MemoryWindows()
        let first = windows.allocWindowId()
        let planned = windows.plan(from: nil, folder: "/work")
        #expect(planned.window != first)
        #expect(windows.allocWindowId() != planned.window)
    }

    @Test func aSavedWindowKeepsItsFrameUntilItIsForgotten() {
        let windows = MemoryWindows()
        let frame = WindowFrame(x: 10, y: 20, width: 900, height: 600, display: 1)
        #expect(windows.save(window: 3, workspace: workspace, frame: frame))
        #expect(windows.records[3] == .some(frame))
        windows.saveFront(3)
        #expect(windows.front == 3)
        windows.forget(3)
        #expect(windows.records[3] == nil && windows.forgotten == [3])
    }

    @Test func theSampleStepsANewWindowAwayFromTheOneItCameFrom() {
        let windows = MemoryWindows()
        let display = DisplayArea(id: 1, x: 0, y: 25, width: 1440, height: 875)
        let next = windows.cascade(WindowFrame(x: 100, y: 100, width: 800, height: 600), on: display)
        #expect((next.x, next.y, next.width, next.display) == (126, 126, 800, 1))
    }

    @Test func theSampleHasNothingSavedAndNothingToHandOver() {
        #expect(MemoryWindows().saved().isEmpty)
        #expect(workspace.takeOut([1]) == nil)
    }
}

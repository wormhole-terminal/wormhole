import Foundation
import TerminalCore
import UniFFISource
import WindowsModel
import WorkspaceModel
import XCTest

@testable import WindowsBridge
@testable import WorkspaceBridge

/// The app's windows over the core's real store, in a data folder of the experiment's own made for each test, with
/// `/bin/cat` as the program of every terminal.
@MainActor
final class WindowsFFITests: XCTestCase {
    private var root: URL!
    private let setup = WorkspaceSetup(program: "/bin/cat", home: "/Users/ref")

    override func setUpWithError() throws {
        root = FileManager.default.temporaryDirectory.appendingPathComponent("windows-bridge-\(UUID().uuidString.prefix(8))")
        let data = root.appendingPathComponent("Wormhole-swift-t")
        try FileManager.default.createDirectory(at: data, withIntermediateDirectories: true)
        setenv("WORMHOLE_DATA_DIR", data.path, 1)
    }

    override func tearDown() async throws { try? FileManager.default.removeItem(at: root) }

    private func window(_ windows: RustWindowsBackend, _ id: Int) -> (WorkspaceStore, RustWorkspaceBackend) {
        let backend = windows.workspace(window: id, setup: setup)
        return (WorkspaceStore(backend: backend), backend)
    }

    func testAWindowComesBackInTheNextRunWithItsTerminalsAndWhereItWas() throws {
        let frame = WindowFrame(x: 120, y: 80, width: 1000, height: 700, display: 1)
        do {
            let windows = try RustWindowsBackend()
            let id = windows.allocWindowId()
            let (store, backend) = window(windows, id)
            store.send(.newTerminal)
            store.send(.newTerminal)
            store.send(.rename(2, "api"))
            store.send(.select(1))
            XCTAssertTrue(windows.save(window: id, workspace: backend, frame: frame))
            windows.saveFront(id)
        }
        let windows = try RustWindowsBackend()
        let saved = windows.saved()
        XCTAssertEqual(saved.map(\.terminals), [2])
        XCTAssertEqual(saved.first?.frame, frame)
        let (store, backend) = window(windows, try XCTUnwrap(saved.first?.id))
        store.restore()
        XCTAssertEqual(store.state.live, [1, 2])
        XCTAssertEqual(store.state.selected, 1)
        XCTAssertEqual(store.state.terminals.first { $0.id == 2 }?.title, "api")
        XCTAssertNotNil(backend.frameSource(for: 1), "its program started again")
        store.send(.newTerminal)
        XCTAssertEqual(store.state.live, [1, 2, 3], "new ids start above the saved ones")
    }

    func testAPlannedWindowOpensWithItsOneTerminalAndForgettingItLeavesNothing() throws {
        let windows = try RustWindowsBackend()
        let planned = windows.plan(from: nil, folder: "/tmp")
        let (store, _) = window(windows, planned.window)
        store.restore()
        XCTAssertEqual(store.state.live, [planned.terminal])
        XCTAssertEqual(store.state.terminals.first?.folder, "/tmp")
        windows.forget(planned.window)
        XCTAssertTrue(windows.saved().isEmpty)
    }

    func testARunningTerminalMovesToAnotherWindowAndGoesOnRunning() throws {
        let windows = try RustWindowsBackend()
        let (first, firstBackend) = window(windows, windows.allocWindowId())
        let (second, secondBackend) = window(windows, windows.allocWindowId())
        first.send(.newTerminal)
        first.send(.newTerminal)
        let oldSource = try XCTUnwrap(firstBackend.frameSource(for: 2))

        let departure = try XCTUnwrap(first.takeOut([2]))
        XCTAssertEqual(departure.ids, [2])
        XCTAssertEqual(first.state.live, [1])
        second.receive(departure)
        XCTAssertEqual(second.state.live, [2])
        XCTAssertEqual(second.state.selected, 2)

        // The old window's view going away must not end the program that moved.
        oldSource.stop()
        let source = try XCTUnwrap(secondBackend.frameSource(for: 2) as? UniFFIFrameSource)
        try source.start(size: .init(cols: 80, rows: 24), wake: {})
        source.write(Data("moved\n".utf8))
        let deadline = Date().addingTimeInterval(10)
        while !source.screenText().contains("moved"), Date() < deadline { RunLoop.current.run(until: Date().addingTimeInterval(0.02)) }
        XCTAssertTrue(source.screenText().contains("moved"))
        XCTAssertNil(first.takeOut([99]))
    }

    func testTheCoreKeepsAWindowReachableAndStepsANewOneAway() throws {
        let windows = try RustWindowsBackend()
        let main = DisplayArea(id: 1, x: 0, y: 25, width: 1440, height: 875)
        let lost = WindowFrame(x: 9000, y: 100, width: 900, height: 600, display: 2)
        XCTAssertEqual(windows.fit(lost, on: [main]).x, 40)
        let kept = WindowFrame(x: 100, y: 100, width: 900, height: 600)
        XCTAssertEqual(windows.fit(kept, on: [main]), kept)
        XCTAssertEqual(windows.cascade(kept, on: main).x, 126)
    }

    func testTheCoreSaysWhereAReleasedTerminalGoes() throws {
        let windows = try RustWindowsBackend()
        let stack = [
            TearOffWindow(window: 2, bounds: CGRect(x: 500, y: 400, width: 600, height: 500)),
            TearOffWindow(window: 1, bounds: CGRect(x: 0, y: 300, width: 800, height: 600)),
            TearOffWindow(window: nil, bounds: CGRect(x: 1200, y: 0, width: 300, height: 300)),
        ]
        func landing(_ x: Double, _ y: Double, covered: Bool = false) -> TearOffLanding {
            windows.tearOffLanding(over: stack, at: CGPoint(x: x, y: y), from: 1, covered: covered)
        }
        XCTAssertEqual(landing(600, 700), .into(2), "the front-most window takes it")
        XCTAssertEqual(landing(100, 700), .stay, "its own window")
        XCTAssertEqual(landing(1300, 100), .stay, "a window that takes nothing hides what lies behind it")
        XCTAssertEqual(landing(1300, 700), .newWindow, "the desktop")
        XCTAssertEqual(landing(600, 700, covered: true), .newWindow, "another application on top")
        let visible = CGRect(x: 0, y: 38, width: 1440, height: 1000)
        let placed = windows.tearOffWindow(at: CGPoint(x: 500, y: 400), size: CGSize(width: 900, height: 600), visible: visible)
        XCTAssertEqual(placed, CGRect(x: 380, y: 310, width: 900, height: 600), "the pointer lies at the new terminal's row")
    }
}

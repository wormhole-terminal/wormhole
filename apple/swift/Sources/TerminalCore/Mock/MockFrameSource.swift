// The synthetic `FrameSource`. In its threaded mode it behaves like the real one where it matters to the
// renderer: a thread of its own produces output, the screen is behind a lock that `frame()` takes only for
// a copy of the damaged rows, and events are one coalesced wake until they are drained. In its manual mode
// (`step`) nothing runs by itself, so a test or a screenshot is deterministic.

import Foundation
import Synchronization

public final class MockFrameSource: FrameSource {
    public struct Configuration: Sendable {
        public var scenario: MockScenario
        public var seed: UInt64
        /// Steps a second; 0 produces as fast as the screen lock lets it (like `ttystorm --fps 0`).
        public var hertz: Double
        /// No thread: the caller drives `step`.
        public var manual: Bool

        public init(scenario: MockScenario = .mixed, seed: UInt64 = 1, hertz: Double = 60, manual: Bool = false) {
            self.scenario = scenario
            self.seed = seed
            self.hertz = hertz
            self.manual = manual
        }
    }

    private struct State {
        var screen: MockScreen
        var rng: SplitMix64
        var tick = 0
    }

    /// The contract's event queue: `Output` collapses, one wake until the next drain.
    private struct Events {
        var pending: [TerminalEvent] = []
        var woken = false
        var wake: (@Sendable () -> Void)?
    }

    private let configuration: Configuration
    private let state: Mutex<State>
    private let events = Mutex<Events>(Events())
    private let running = Atomic<Bool>(false)
    private let stepsTaken = Atomic<Int>(0)
    private let framesTaken = Atomic<Int>(0)

    public init(_ configuration: Configuration = .init(), size: GridSize = GridSize(cols: 80, rows: 24)) {
        self.configuration = configuration
        var screen = MockScreen(cols: size.cols, rows: size.rows)
        configuration.scenario.prepare(&screen)
        state = Mutex(State(screen: screen, rng: SplitMix64(seed: configuration.seed)))
    }

    /// Steps produced and frames taken so far (for the measuring mode's report).
    public var counters: (steps: Int, frames: Int) { (stepsTaken.load(ordering: .relaxed), framesTaken.load(ordering: .relaxed)) }

    // MARK: lifecycle

    public func start(size: GridSize, wake: @escaping @Sendable () -> Void) throws {
        events.withLock {
            $0.wake = wake
            $0.pending.append(.title("Mock: \(configuration.scenario.rawValue)"))
        }
        state.withLock { $0.screen.resize(cols: size.cols, rows: size.rows) }
        notify(.output)
        guard !configuration.manual, !running.exchange(true, ordering: .acquiringAndReleasing) else { return }
        let source = self
        let thread = Thread { source.generate() }
        thread.name = "mock-frame-source"
        thread.qualityOfService = .userInitiated
        thread.start()
    }

    public func stop() {
        running.store(false, ordering: .releasing)
        events.withLock { $0.wake = nil }
    }

    /// One step of the scenario; in the threaded mode the thread calls this.
    public func step() {
        state.withLock { state in
            state.tick += 1
            configuration.scenario.step(&state.screen, rng: &state.rng, tick: state.tick)
            state.screen.snapToLive()
        }
        stepsTaken.add(1, ordering: .relaxed)
        notify(.output)
    }

    private func generate() {
        let interval = configuration.hertz > 0 ? 1 / configuration.hertz : 0
        var next = ContinuousClock.now
        while running.load(ordering: .acquiring) {
            step()
            if interval > 0 {
                next += .seconds(interval)
                // A deadline in the past (the process was starved) must not make the thread catch up in a burst.
                if next < .now { next = .now }
                Thread.sleep(until: Date(timeIntervalSinceNow: (next - .now).seconds))
            } else {
                // Flat out, but give the main thread's `frame()` a chance at the lock: the real screen's lock is
                // a fair one for the same reason.
                usleep(20)
            }
        }
    }

    // MARK: FrameSource

    public func frame() -> Data {
        framesTaken.add(1, ordering: .relaxed)
        return state.withLock { $0.screen.takeFrame(all: false) }
    }

    public func fullFrame() -> Data { state.withLock { $0.screen.takeFrame(all: true) } }

    public func drainEvents() -> [TerminalEvent] {
        events.withLock { events in
            events.woken = false
            let taken = events.pending
            events.pending.removeAll(keepingCapacity: true)
            return taken
        }
    }

    public func write(_ bytes: Data) {
        guard let text = String(data: bytes, encoding: .utf8) else { return }
        state.withLock { state in
            state.screen.snapToLive()
            // A shell echoes typed text; the mock does it without the shell.
            state.screen.pen = PackedCell()
            state.screen.write(text.replacingOccurrences(of: "\r", with: "\r\n"))
        }
        if text.unicodeScalars.contains(where: { $0.value == 7 }) { notify(.bell) }
        notify(.output)
    }

    public func resize(_ size: GridSize) {
        state.withLock { $0.screen.resize(cols: size.cols, rows: size.rows) }
        notify(.output)
    }

    public func scroll(lines: Int) {
        state.withLock { $0.screen.scrollView(lines: lines) }
        notify(.output)
    }

    private let pointerAndSelection = Mutex<MockInteraction?>(nil)

    /// One object for the source's life: it holds the selection.
    public var interaction: (any TerminalInteraction)? {
        pointerAndSelection.withLock { made in
            if made == nil { made = MockInteraction { [unowned self] in self.state.withLock { $0.screen.visibleText() } } }
            return made
        }
    }

    // MARK: events

    private func notify(_ event: TerminalEvent) {
        let wake: (@Sendable () -> Void)? = events.withLock { events in
            // Output is "something is new", not a count: one is enough until it is drained.
            if event == .output, events.pending.contains(.output) { return nil }
            events.pending.append(event)
            guard !events.woken else { return nil }
            events.woken = true
            return events.wake
        }
        // Called with no lock held, as the contract says.
        wake?()
    }
}

extension Duration {
    fileprivate var seconds: Double {
        let (s, atto) = components
        return Double(s) + Double(atto) / 1e18
    }
}

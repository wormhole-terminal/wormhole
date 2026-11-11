// What the question before quitting is decided from (`window::windows::quit_asking`): the terminals that run
// something in every window, how many of them are kept by a session host and how many run inside the app, the
// programs that would end, and what other profiles still run.

import Foundation

public struct QuitFacts: Equatable, Sendable {
    public var running: [StillRunning.Fact]
    public var programs: [String]
    public var otherProfiles: [(name: String, count: Int)]
    public var hostedRunning: Int
    public var localRunning: Int
    /// Whether any terminal at all is hosted (even idle): the box has something to keep.
    public var anyHosted: Bool

    public init(
        running: [StillRunning.Fact] = [], programs: [String] = [], otherProfiles: [(name: String, count: Int)] = [],
        hostedRunning: Int = 0, localRunning: Int = 0, anyHosted: Bool = false
    ) {
        (self.running, self.programs, self.otherProfiles) = (running, programs, otherProfiles)
        (self.hostedRunning, self.localRunning, self.anyHosted) = (hostedRunning, localRunning, anyHosted)
    }

    public static func == (a: QuitFacts, b: QuitFacts) -> Bool {
        a.running == b.running && a.programs == b.programs && a.hostedRunning == b.hostedRunning
            && a.localRunning == b.localRunning && a.anyHosted == b.anyHosted
            && a.otherProfiles.map(\.name) == b.otherProfiles.map(\.name) && a.otherProfiles.map(\.count) == b.otherProfiles.map(\.count)
    }

    /// The question to put, or `nil` when nothing runs and the app just quits. `endAll` is "Quit and End All Terminals".
    public func question(endAll: Bool) -> Question? {
        guard let ask = QuitQuestion.ask(endHosts: endAll, hostedRunning: hostedRunning, localRunning: localRunning, anyHosted: anyHosted)
        else { return nil }
        let detail = QuitQuestion.detail(
            running: running, programs: programs, otherProfiles: otherProfiles,
            hostedRunning: ask == .plain ? 0 : hostedRunning, localRunning: ask == .plain ? 0 : localRunning)
        return QuitQuestion.question(ask: ask, detail: detail, endAll: endAll)
    }

    /// Whether the session hosts end with the app: always for End All; otherwise the box unticked.
    public static func endsHosts(endAll: Bool, answer: Answer) -> Bool {
        if endAll { return true }
        if case .confirm(let kept) = answer { return QuitQuestion.endsHosts(keepRunning: kept) }
        return false
    }
}

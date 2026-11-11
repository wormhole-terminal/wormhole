// The first-run flow as data (`onboarding::Onboarding`): which step is showing, where Back, Continue, Skip and
// Start lead, and what the app is asked to do. Nothing is installed or trusted on the way through: hooks are
// written only for the agent whose own Install button was pressed, and a folder is trusted only from its own
// button. Navigation never yields either action (the tests drive every path).

import Foundation

public enum OnboardingEvent: Equatable, Sendable {
    case `continue`, back, skip, start
    case pickFolder(URL)
    case install(String)
    case trust(URL)
}

public enum OnboardingAction: Equatable, Sendable {
    case none, skip
    case finish(folder: URL?)
    case installHooks(String)
    case trust(URL)
}

public struct OnboardingFlow: Equatable, Sendable {
    public private(set) var step: OnboardingStep = .welcome
    public private(set) var folder: URL?
    private var agentsFound: Bool
    private let replay: Bool

    public init(agentsFound: Bool) { self.agentsFound = agentsFound; replay = false }

    /// A replay of the intro later: the welcome page alone.
    public static func replay() -> OnboardingFlow { OnboardingFlow(agentsFound: true, replay: true) }

    private init(agentsFound: Bool, replay: Bool) { (self.agentsFound, self.replay) = (agentsFound, replay) }

    public var isLast: Bool { step.isLast || (replay && step == .welcome) }

    public mutating func toWelcome() { step = .welcome }

    /// The agents turned up (or didn't) after the search finished: the third step follows.
    public mutating func setAgentsFound(_ found: Bool) {
        agentsFound = found
        if step.isLast { step = third }
    }

    private var third: OnboardingStep { agentsFound ? .agents : .primer }

    public mutating func apply(_ event: OnboardingEvent) -> OnboardingAction {
        switch event {
        case .continue where isLast && step == .welcome:
            return .finish(folder: nil)
        case .continue:
            switch step {
            case .welcome: step = .folder
            case .folder: step = .look
            case .look: step = third
            case .agents, .primer: break
            }
            return .none
        case .back:
            switch step {
            case .welcome, .folder: step = .welcome
            case .look: step = .folder
            case .agents, .primer: step = .look
            }
            return .none
        case .skip: return .skip
        case .start where isLast: return .finish(folder: folder)
        // Start belongs to the last step; anywhere else it is a Continue.
        case .start: return apply(.continue)
        case .pickFolder(let url):
            folder = url
            return .none
        case .install(let id): return step == .agents ? .installHooks(id) : .none
        case .trust(let url): return step == .agents ? .trust(url) : .none
        }
    }
}

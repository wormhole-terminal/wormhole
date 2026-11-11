// A step of the first-run flow (`onboarding::Step`): the intro window's welcome page, then three steps.

import Foundation

public enum OnboardingStep: Equatable, Sendable {
    /// The intro window's welcome page, before the three steps.
    case welcome
    /// "Where do you work?": the folder the first terminal starts in.
    case folder
    /// "Make it yours": theme, primary, font.
    case look
    /// "Coding agents": the agents on PATH, each with its own Install, and the folder's trust.
    case agents
    /// "Ready when you are": no agent on PATH, so a primer instead.
    case primer

    public static let counted = 3

    /// The step's place, 1-based, of the three the flow counts; the welcome page is none of them.
    public var number: Int {
        switch self {
        case .welcome: 0
        case .folder: 1
        case .look: 2
        case .agents, .primer: 3
        }
    }

    /// The last step: its main button says Start.
    public var isLast: Bool { self == .agents || self == .primer }
}

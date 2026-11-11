// The intro window's state while it is open (`onboarding::Intro`): the flow, what was found on PATH, the folders the
// first step offers, and what each button's action does through the backend. It never reads the app around it: the
// backend tells it, and it ends by calling `onEnd` (a first run is remembered and the first terminal moves to the
// folder picked; a replay changes nothing).

import Foundation
import Observation

@MainActor @Observable
public final class OnboardingSession {
    public private(set) var flow: OnboardingFlow
    /// `nil` until the agents were looked for.
    public private(set) var agents: [FoundAgent]?
    public private(set) var othersFound = 0
    public private(set) var places: [Place]
    /// "Later" was pressed on the folder's trust.
    public private(set) var trustLater = false
    /// What went wrong writing an agent's hooks.
    public private(set) var problem: String?
    /// The first run, or a replay of the intro, which changes nothing.
    public let firstRun: Bool
    @ObservationIgnored public var onEnd: ((_ folder: URL?, _ firstRun: Bool) -> Void)?
    @ObservationIgnored public let backend: any OnboardingBackend
    /// The Look step's import offer and "How much to show" walk.
    public let look: LookExtras

    public init(backend: any OnboardingBackend, firstRun: Bool) {
        self.backend = backend
        look = LookExtras(backend: backend)
        self.firstRun = firstRun
        flow = firstRun ? OnboardingFlow(agentsFound: true) : .replay()
        places = backend.places()
    }

    public var step: OnboardingStep { flow.step }
    public var preferences: OnboardingPreferences { backend.preferences }
    public var trustCard: TrustCard? { trustLater ? nil : backend.trustCard }

    /// Looks for agents off the main thread, then lets the third step follow what was found.
    public func scan() async {
        let (found, others) = await backend.scanAgents()
        agents = found
        othersFound = others
        flow.setAgentsFound(!found.isEmpty)
    }

    public func send(_ event: OnboardingEvent) {
        switch flow.apply(event) {
        case .none: break
        case .skip: onEnd?(nil, firstRun)
        case .finish(let folder): onEnd?(folder, firstRun)
        case .installHooks(let id): Task { await install(id) }
        case .trust(let folder): backend.trust(folder)
        }
    }

    public func replay() { flow.toWelcome() }
    public func laterOnTrust() { trustLater = true }

    /// "Open folder…" or a folder dropped on the card.
    public func chooseFolder() async {
        if let folder = await backend.chooseFolder() { send(.pickFolder(folder)) }
    }

    public func pick(_ folder: URL) { send(.pickFolder(folder)) }


    private func install(_ id: String) async {
        do {
            let updated = try await backend.installHooks(agent: id)
            problem = nil
            if let index = agents?.firstIndex(where: { $0.id == id }) { agents?[index] = updated }
        } catch {
            problem = error.localizedDescription
        }
    }
}

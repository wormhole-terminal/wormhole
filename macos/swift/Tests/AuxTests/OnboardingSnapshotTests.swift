import AuxModel
import DesignSystem
import XCTest
@testable import AuxWindows

@MainActor
final class OnboardingSnapshotTests: XCTestCase {
    static let themes = ["wormhole.dark", "tokyo_night", "wormhole.light", "catppuccin_latte"]

    private func page(_ step: OnboardingStep, agents: Bool = true) async -> OnboardingView {
        let backend = SampleOnboardingBackend(agents: agents ? SampleOnboardingBackend.someAgents : [])
        let session = OnboardingSession(backend: backend, firstRun: true)
        await session.scan()
        var event: [OnboardingEvent] = []
        switch step {
        case .welcome: break
        case .folder: event = [.continue]
        case .look: event = [.continue, .continue]
        case .agents, .primer: event = [.continue, .continue, .continue]
        }
        for e in event { session.send(e) }
        return OnboardingView(session: session, player: IntroPlayer(reduceMotion: { true }), replay: {})
    }

    func testPages() async throws {
        for theme in Self.themes {
            for (name, step, agents) in [
                ("onboarding-welcome", OnboardingStep.welcome, true), ("onboarding-folder", .folder, true),
                ("onboarding-look", .look, true), ("onboarding-agents", .agents, true), ("onboarding-primer", .primer, false),
            ] {
                try Snapshot.write(await page(step, agents: agents), name: name, theme: theme)
            }
        }
        try Snapshot.write(await page(.agents), name: "onboarding-agents", theme: "wormhole.dark", contrast: true)
    }
}

import XCTest

@testable import AuxModel

/// The Look step's offer and walk, over a backend that answers from fixed data.
@MainActor
final class LookExtrasTests: XCTestCase {
    private final class Walk: TailorWalking {
        var acts: [TailorAct] = []
        func view(width: Double) -> TailorView {
            TailorView(
                stage: .asking, heading: "How much to show", question: "Where do you want to start?", counter: "1 of 7", detail: "",
                summaryTitle: "", settledText: "", backLabel: "Back", startOverLabel: "Start over", looksGoodLabel: "Looks good",
                announcement: "", answers: [], summary: [], canBack: false, panelOpen: acts.contains(.answer(2)))
        }
        func act(_ act: TailorAct) -> Bool { acts.append(act); return true }
    }

    private final class Backend: OnboardingBackend {
        let walk = Walk()
        var panel: [Bool] = []
        private let sample = SampleOnboardingBackend()
        var preferences: OnboardingPreferences { sample.preferences }
        var trustCard: TrustCard? { nil }
        var settingsKeys: String { "" }
        func scanAgents() async -> (found: [FoundAgent], others: Int) { ([], 0) }
        func installHooks(agent: String) async throws -> FoundAgent { throw CocoaError(.fileNoSuchFile) }
        func trust(_ folder: URL) {}
        func places() -> [Place] { [] }
        func chooseFolder() async -> URL? { nil }
        func finish(folder: URL?, firstRun: Bool) {}
        func tailor() -> (any TailorWalking)? { walk }
        func panelOpen(_ open: Bool) { panel.append(open) }
        func importOffer() async -> ImportChoice? { ImportChoice(source: "ghostty", offer: "Use my Ghostty settings") }
    }

    func testTheStepBeginsTheWalkAndScansForAnOfferOnce() async {
        let backend = Backend()
        let look = LookExtras(backend: backend)
        await look.begin(width: 100)
        XCTAssertEqual(look.walk?.question, "Where do you want to start?")
        XCTAssertEqual(look.choice?.offer, "Use my Ghostty settings")
        await look.begin(width: 100)
        XCTAssertEqual(backend.walk.acts, [], "a second draw begins nothing again")
    }

    func testAnAnswerIsGivenToTheWalkAndTheWindowsPanelFollowsIt() async {
        let backend = Backend()
        let look = LookExtras(backend: backend)
        await look.begin(width: 100)
        look.act(.answer(2))
        XCTAssertEqual(backend.walk.acts, [.answer(2)])
        XCTAssertEqual(backend.panel, [true], "the walk says the panel is open; the window's own layout is the app's to change")
    }
}

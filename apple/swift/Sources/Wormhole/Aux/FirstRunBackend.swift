// The welcome's backend: the sample one's agents and folders (the core supplies the real scan through the FFI)
// with the one thing that is real already: finishing a first run writes a marker in the data folder, so the
// welcome shows by itself only once there.

import AuxModel
import Foundation

@MainActor
final class FirstRunBackend: OnboardingBackend {
    private let sample = SampleOnboardingBackend()
    private let marker: URL

    init(marker: URL) { self.marker = marker }

    static func marker(in dataDirectory: URL) -> URL { dataDirectory.appendingPathComponent("welcomed") }

    var preferences: OnboardingPreferences { sample.preferences }
    var trustCard: TrustCard? { sample.trustCard }
    var settingsKeys: String { sample.settingsKeys }
    func scanAgents() async -> (found: [FoundAgent], others: Int) { await sample.scanAgents() }
    func installHooks(agent: String) async throws -> FoundAgent { try await sample.installHooks(agent: agent) }
    func trust(_ folder: URL) { sample.trust(folder) }
    func places() -> [Place] { sample.places() }
    func chooseFolder() async -> URL? { await sample.chooseFolder() }

    func finish(folder: URL?, firstRun: Bool) {
        sample.finish(folder: folder, firstRun: firstRun)
        if firstRun { try? Data().write(to: marker) }
    }
}

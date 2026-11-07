// A refusal from the settings seam as the sentence core wrote for it.

import WormholeBindings

enum SettingsFailure {
    static func sentence(_ error: any Error) -> String {
        switch error as? SettingsError {
        case .NotIsolated(let reason), .Invalid(let reason), .Profile(let reason), .Agents(let reason): reason
        case nil: String(describing: error)
        }
    }
}

/// A sentence thrown across the model's seam, which prints as the sentence and nothing else.
struct Sentence: Error, CustomStringConvertible {
    let description: String
    init(_ error: any Error) { description = SettingsFailure.sentence(error) }
}

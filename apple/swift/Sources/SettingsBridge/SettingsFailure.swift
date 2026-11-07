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

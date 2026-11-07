// How the sidebar speaks and draws a terminal's activity. The design system's dot knows no terminals,
// so the mapping lives here.

import DesignSystem
import WorkspaceModel

extension Activity {
    var dot: StatusKind {
        switch self {
        case .idle: .idle
        case .working: .working
        case .needsInput: .attention
        case .finished: .done
        case .failed: .failed
        }
    }

    /// The state in words, for VoiceOver (the dot itself is hidden from it).
    var spoken: String {
        switch self {
        case .idle: "idle"
        case .working: "working"
        case .needsInput: "needs input"
        case .finished: "finished"
        case .failed: "failed"
        }
    }
}

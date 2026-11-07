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
}

// What a row's "±N" says (`sidebar/row/face.rs` `changes_above`, `sidebar_host.rs` `row_changes`): by Settings → Changed files
// → Count, the files this terminal changed (the core's `changeCount`), or the repository's, which the list shows once, on
// the topmost row of that repository.

import SwiftUI
import WorkspaceModel

/// Which files a row's count is of (`sidebar_changes_scope`).
public enum ChangesScope: String, Sendable {
    case terminal, repository
}

enum RowChanges {
    /// The number each row shows; a row with none is not in the result.
    static func counts(_ sections: [SidebarSection], scope: ChangesScope) -> [TerminalID: Int] {
        var counts: [TerminalID: Int] = [:]
        var seen = Set<String>()
        for terminal in sections.flatMap(\.rows).map(\.terminal) {
            switch scope {
            case .terminal:
                if terminal.changeCount > 0 { counts[terminal.id] = terminal.changeCount }
            case .repository:
                guard let repo = terminal.repo, seen.insert(repo).inserted, let files = terminal.repoFiles, files > 0 else { continue }
                counts[terminal.id] = files
            }
        }
        return counts
    }
}

extension EnvironmentValues {
    /// The "±N" of each row, as `RowChanges` counted them for the list in view.
    @Entry var rowChanges: [TerminalID: Int] = [:]
}

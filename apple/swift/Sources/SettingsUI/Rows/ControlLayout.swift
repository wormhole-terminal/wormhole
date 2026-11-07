// Where a control sits: at the right of its row, in a block of its own (no row around it), or in the preview
// strip above the page. The page decides from this and never from the kind's name.

import SettingsModel

enum ControlLayout {
    case row
    /// A row whose control runs over more than one line: the label sits beside its first line.
    case tallRow
    /// Content that stands on its own on the card, with no label column.
    case block
    /// The scene of the preview strip: no row.
    case strip
    /// The page's whole body, in cards of its own: no card around it (Project folders).
    case bare
}

extension ControlSpec {
    /// Row panels whose control runs over more than one line.
    private static let tallPanels: Set<String> = ["profile-name", "profile-start", "profile-env", "profile-logins"]

    var layout: ControlLayout {
        switch self {
        case .preview: .strip
        case .themeCards: .block
        case .panel(let name):
            if name == "projects" { .bare } else if !isRowPanel { .block } else if Self.tallPanels.contains(name) { .tallRow } else { .row }
        case .partsList: .tallRow
        default: .row
        }
    }
}

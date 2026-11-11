// One line of the Changes list, as `features/git/changes/rows.rs` builds it: a card head per section, folder
// rows in tree mode, files, and the spacer that lets the last row scroll clear of the panel's edge.

import WordsModel

public enum ChangeGroup: String, Sendable {
    case all, changes, untracked

    /// Without grouping every file is a change.
    public var title: String {
        self == .untracked
            ? Words.text("changes-group-untracked", fallback: "Untracked") : Words.text("changes-group-changes", fallback: "Changes")
    }
}

public enum ChangeRow: Identifiable, Equatable, Sendable {
    case band(group: ChangeGroup, count: Int, folded: Bool, first: Bool)
    case foldNote(String)
    case directory(group: ChangeGroup, path: String, label: String, depth: Int, collapsed: Bool)
    case file(ChangedFile, depth: Int, inTree: Bool)
    case spacer

    public var id: String {
        switch self {
        case let .band(group, _, _, _): "band-\(group.rawValue)"
        case .foldNote: "fold-note"
        case let .directory(group, path, _, _, _): "dir-\(group.rawValue)-\(path)"
        case let .file(file, depth, _): "file-\(depth)-\(file.path)"
        case .spacer: "spacer"
        }
    }
}

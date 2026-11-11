// A folder's colour as the Project tab's row shows it (`features/project/folder_color`): which of the eight names (or
// a hex colour set by hand) is chosen, whose it is and where it is kept. The line under the row says which, and a
// project that locks its colour shows the row dimmed.

import WordsModel
import WorkspaceModel

public struct FolderPaint: Equatable, Sendable {
    public var chosen: FolderColor?
    /// A colour the project's config.json sets by hand, matching none of the names: "#7fb2ff".
    public var custom: String?
    /// The project's config.json locks the colour, or can't be read: the row cannot change it.
    public var locked: Bool
    /// The line under the row.
    public var note: String

    public init(chosen: FolderColor? = nil, custom: String? = nil, locked: Bool = false, note: String) {
        (self.chosen, self.custom, self.locked, self.note) = (chosen, custom, locked, note)
    }

    public var enabled: Bool { !locked }

    /// Where an own colour is kept: yours only.
    public static var own: FolderPaint {
        FolderPaint(note: Words.text("project-color-own", fallback: "Marks this folder's band. Yours only: it isn't saved in the project."))
    }
}

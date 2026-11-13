// What the title row's controls do that the row cannot: each is the window's to carry out.

import Foundation
import PanesModel
import SwiftUI
import WorkspaceModel

public struct TitleActions {
    /// The folder's path itself in Finder (a folder is opened, a file shown).
    public var revealFolder: () -> Void
    public var copyPath: () -> Void
    /// Types the path, quoted for the shell, at the focused terminal's prompt as a paste.
    public var insertPath: () -> Void
    public var pickBranch: () -> Void
    /// The branch status's click: the Changes tab, where what changed is.
    public var showChanges: () -> Void
    public var split: (SplitAxis) -> Void
    public var canSplit: (SplitAxis) -> Bool
    public var setPart: (TitlePart, Bool) -> Void
    public var resetRow: () -> Void
    public var customizeRow: () -> Void
    /// Whether the terminal has a shell to type into (an exited one has none).
    public var canInsert: Bool
    /// Whether this window may arrange the row: not under Zen mode (the row on screen is Zen's) and not in a window of a
    /// profile that is not the shown one (Settings shows the shown profile's).
    public var canEditRow = true
    /// The terminal's tile as its row in the list draws it (the agent's logo, the status light); nil draws the plain tile.
    public var tile: ((TerminalID, CGFloat) -> AnyView)?

    public init(
        revealFolder: @escaping () -> Void = {}, copyPath: @escaping () -> Void = {}, insertPath: @escaping () -> Void = {},
        pickBranch: @escaping () -> Void = {}, showChanges: @escaping () -> Void = {},
        split: @escaping (SplitAxis) -> Void = { _ in }, canSplit: @escaping (SplitAxis) -> Bool = { _ in true },
        setPart: @escaping (TitlePart, Bool) -> Void = { _, _ in }, resetRow: @escaping () -> Void = {},
        customizeRow: @escaping () -> Void = {}, canInsert: Bool = true
    ) {
        (self.revealFolder, self.copyPath, self.insertPath, self.pickBranch, self.showChanges) = (revealFolder, copyPath, insertPath, pickBranch, showChanges)
        (self.split, self.canSplit, self.setPart, self.resetRow, self.customizeRow, self.canInsert) = (split, canSplit, setPart, resetRow, customizeRow, canInsert)
    }
}

// What the person chose about how the list looks, which the model does not carry: Settings → Terminals & sidebar
// (Density, Show groups as cards, Fade group colours, Show group folders, notes and changes on rows) and Motion →
// Status. The window passes it in; the defaults are the shipping app's (`settings` defaults).

import DesignSystem
import SwiftUI
import WorkspaceModel

/// How a terminal's status change is played (`StatusMotion`, `view/motion/choices.rs`). The GPUI app's default is a cut
/// (Classic); here the badge grows in on a spring, which is that app's "Badge pops" with a native spring.
public enum StatusMotion: String, Sendable, CaseIterable {
    /// The tile's light, edge and badge change at once, and so do the words.
    case classic
    /// The old light, edge and badge fade out while the new ones fade in.
    case crossfade
    /// The corner badge grows in past its size and settles.
    case badgePop
    /// The card is washed in the new status's colour, which fades away.
    case wash
    /// The second line rolls up to its new words, like a counter.
    case lineRoll
}

public struct SidebarAppearance: Equatable {
    public var density: SidebarDensity
    /// Terminals and groups stand in cards (`show_groups_as_cards`); off they run the sidebar's width.
    public var cards: Bool
    /// A group's wash fades from its left edge instead of filling the card evenly (`fade_group_colors`).
    public var fadesGroupColours: Bool
    /// A group with a folder shows it under its name (`show_group_folders`).
    public var showsGroupFolders: Bool
    /// The open notes and tasks, and the changed files, at the end of a row's first line (`sidebar_notes`, `sidebar_changes`).
    public var showsNotes: Bool
    public var showsChanges: Bool
    /// Whose files the "±N" counts (`sidebar_changes_scope`).
    public var changesScope: ChangesScope
    /// The tile says what its terminal is doing; off, every tile is the quiet one (`sidebar_status_dot`).
    public var showsStatus: Bool
    /// ⌘ held on its own for a moment shows the ⌘1–9 keycaps (`sidebar_cmd_hints`).
    public var showsCommandHints: Bool
    public var statusMotion: StatusMotion
    /// How a row comes and goes, and how a group folds (`motion.insert`, `motion.fold`: the core's names, its tables are `RowMotion`).
    public var insertMotion: String
    public var foldMotion: String
    /// Terminal text is kept between launches (`restore_terminal_text`): while it is not, "Don't keep this terminal's text" is greyed.
    public var restoresText: Bool
    /// "Hide from Agents" is in a row's menu: with the Automation feature on (or while the terminal is hidden, to show it again).
    public var offersAgentHide = false
    /// Shows the ⌘1–9 keycaps whatever ⌘ is doing: for pictures and tests.
    public var forcesHints: Bool
    /// The row drawn as if the pointer were over it: for pictures and tests.
    public var hovered: TerminalID?

    public init(
        density: SidebarDensity = .regular, cards: Bool = true, fadesGroupColours: Bool = false,
        showsGroupFolders: Bool = true, showsNotes: Bool = true, showsChanges: Bool = true,
        changesScope: ChangesScope = .terminal, showsStatus: Bool = true, showsCommandHints: Bool = true,
        statusMotion: StatusMotion = .badgePop, insertMotion: String = "from-side", foldMotion: String = "tuck", restoresText: Bool = true, forcesHints: Bool = false, hovered: TerminalID? = nil
    ) {
        self.density = density
        self.cards = cards
        self.fadesGroupColours = fadesGroupColours
        self.showsGroupFolders = showsGroupFolders
        self.showsNotes = showsNotes
        self.showsChanges = showsChanges
        self.changesScope = changesScope
        self.showsStatus = showsStatus
        self.showsCommandHints = showsCommandHints
        self.statusMotion = statusMotion
        self.insertMotion = insertMotion
        self.foldMotion = foldMotion
        self.restoresText = restoresText
        self.forcesHints = forcesHints
        self.hovered = hovered
    }
}

extension EnvironmentValues {
    /// What the list's rows measure at the chosen density, read by every part of it.
    @Entry var sidebarMetrics = SidebarMetrics()
    /// What the person chose; the rows read the parts that are theirs.
    @Entry var sidebarLook = SidebarAppearance()
    /// The ⌘N keycaps are showing (⌘ held on its own for a moment).
    @Entry var showsHints = false
    /// The stop the keys are on, while the list has the keyboard: it wears the focus ring.
    @Entry var keyedStop: SidebarStop?
}

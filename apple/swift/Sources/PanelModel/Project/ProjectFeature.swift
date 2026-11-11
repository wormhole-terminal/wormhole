// The parts a `.wormhole/` project folder can offer and the person switches on or off per project
// (`wormhole_core::project::Feature`): the labels, the file behind each and the glyph in front of its switch.

import WordsModel

public enum ProjectFeature: String, CaseIterable, Sendable {
    case settings, theme, actions, prompts, layout, notes

    public var label: String {
        switch self {
        case .settings: Words.text("project-feature-settings", fallback: "Settings")
        case .theme: Words.text("project-feature-theme", fallback: "Theme")
        case .actions: Words.text("project-feature-actions", fallback: "Actions")
        case .prompts: Words.text("project-feature-prompts", fallback: "Prompts")
        case .layout: Words.text("project-feature-layout", fallback: "Terminal layout")
        case .notes: Words.text("project-feature-notes", fallback: "Shared notes")
        }
    }

    public var detail: String {
        switch self {
        case .settings:
            Words.text("project-feature-settings-detail", fallback: "Setting overrides, environment variables and shell from config.json")
        case .theme: Words.text("project-feature-theme-detail", fallback: "The project's color theme from theme.json")
        case .actions: Words.text("project-feature-actions-detail", fallback: "Commands and key bindings from actions.json")
        case .prompts: Words.text("project-feature-prompts-detail", fallback: "Reusable prompts from the prompts folder")
        case .layout:
            Words.text(
                "project-feature-layout-detail",
                fallback: "Terminals opened (and commands started) with the project, from layout.json")
        case .notes: Words.text("project-feature-notes-detail", fallback: "Notes and next steps synced with notes.md")
        }
    }

    /// Lucide's glyphs in the system's symbols.
    public var symbol: String {
        switch self {
        case .settings: "gearshape"
        case .theme: "paintpalette"
        case .actions: "play"
        case .prompts: "doc.text"
        case .layout: "terminal"
        case .notes: "doc.badge.plus"
        }
    }
}

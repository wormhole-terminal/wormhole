// The parts a `.wormhole/` project folder can offer and the person switches on or off per project
// (`wormhole_core::project::Feature`): the labels, the file behind each and the glyph in front of its switch.

public enum ProjectFeature: String, CaseIterable, Sendable {
    case settings, theme, actions, prompts, layout, notes

    public var label: String {
        switch self {
        case .settings: "Settings"
        case .theme: "Theme"
        case .actions: "Actions"
        case .prompts: "Prompts"
        case .layout: "Terminal layout"
        case .notes: "Shared notes"
        }
    }

    public var detail: String {
        switch self {
        case .settings: "Setting overrides, environment variables and shell from config.json"
        case .theme: "The project's color theme from theme.json"
        case .actions: "Commands and key bindings from actions.json"
        case .prompts: "Reusable prompts from the prompts folder"
        case .layout: "Terminals opened (and commands started) with the project, from layout.json"
        case .notes: "Notes and next steps synced with notes.md"
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

// Where an opened diff is shown. The GPUI app lays it over the terminal's own surface (the panel keeps the list);
// here it can also share the panel with the list. The window reads this and hosts `PanelCoverView` over the terminal
// column for `.terminalArea`.

public enum DiffPresentation: String, Sendable, CaseIterable {
    /// Under the list, in the panel.
    case panel
    /// In place of the terminal, the panel's list taking the whole panel.
    case terminalArea
}

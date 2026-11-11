// The tabs of the right panel, in the order the GPUI app lists them (`config::tab::TabId::ALL`). The window owns
// the strip; this names what is inside each. The raw values are the strings a saved window layout holds.

public enum PanelTab: String, CaseIterable, Identifiable, Sendable {
    case changes, history, notes, usage, project, files

    public var id: String { rawValue }
}

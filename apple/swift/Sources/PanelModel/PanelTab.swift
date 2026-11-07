// The three tabs of the right panel. The window owns the strip; this names what is inside each.

public enum PanelTab: String, CaseIterable, Identifiable, Sendable {
    case notes, changes, history

    public var id: String { rawValue }
}

// One of the three fonts a picker sets (`theme::fonts::FontSlot`): what it is called, what an empty family
// follows, and the sizes it comes in. The schema's font control carries the same facts (`ControlSpec.FontSlot`).

public struct FontPickerSlot: Sendable, Equatable {
    public let key: String
    /// What an empty family follows ("Same as terminal"); nil for the terminal font, which always names a family.
    public let inheritLabel: String?
    public let sizes: ClosedRange<Double>

    public init(key: String, inheritLabel: String?, sizes: ClosedRange<Double>) {
        (self.key, self.inheritLabel, self.sizes) = (key, inheritLabel, sizes)
    }

    /// Terminal and code text line up in columns, so their pickers start with monospace fonts only
    /// (`FontSlot::prefers_monospace`).
    public var prefersMonospace: Bool { key != "interface" }
}

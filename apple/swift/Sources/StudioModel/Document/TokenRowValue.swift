// What a token's row draws: a swatch (or a corner for a metric) and the value beside it, as typed when the theme
// sets it itself, else what it works out to (`studio/token_rows.rs`).

import DesignSystem

public struct TokenRowValue: Sendable, Equatable {
    public enum Chip: Sendable, Equatable { case color(RGBA), metric(Double) }

    public let chip: Chip
    public let text: String
}

public enum NumberText {
    /// A metric as the editor shows it: whole numbers without a decimal point.
    public static func pixels(_ value: Double) -> String { value == value.rounded() ? String(Int(value)) : String(value) }
}

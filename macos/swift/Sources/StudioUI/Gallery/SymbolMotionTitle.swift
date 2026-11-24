// What each motion says of itself on the SF Symbols page: what moves, how long it takes, and which of today's motions it
// carries over (the numbers are `IconMotion.Seconds` and `ColumnSlide`'s).
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem

extension SymbolMotion {
    var title: String {
        switch self {
        case .quarter: "turns a quarter, 220 ms (today's +)"
        case .spin(let clockwise): "turns once \(clockwise ? "clockwise" : "back"), 520 ms (today's arrows)"
        case .copied: "sheet fills, becomes a check, 1.4 s (today's copy)"
        case .paneFill(let half): "new \(half == .right ? "right" : "lower") half grows from the divider, 480 ms (today's split)"
        case .pulse(let edge): "fills from the \(Self.words(edge)), 480 ms, lets go"
        case .disc: "a disc grows from the centre, 480 ms, lets go"
        case .written(let edge): "drawn from the \(Self.words(edge)), 300 ms"
        case .lines: "lines written one after the other, 480 ms (today's notes)"
        case .around: "circle drawn round, 480 ms"
        case .needle: "needle swept up from far back, 520 ms (today's gauge)"
        case .flash: "flashes full and drains, 420 ms (today's folder)"
        case .strip(let edge): "strip fills from the \(Self.words(edge)) with its column, 250 ms in, 200 out (today)"
        case .wipe(let edge): "fill wipes in from the \(Self.words(edge)), 250 ms in, 200 out"
        case .grow: "a disc grows from the centre, 250 ms in, 200 out"
        case .swap: "shrinks away for the other, 240 ms (today's swap)"
        case .turn(let degrees): "turns over \(Int(degrees))°, 200 ms (today's chevron)"
        }
    }

    private static func words(_ edge: Edge) -> String {
        switch edge {
        case .leading: "left"
        case .trailing: "right"
        case .top: "top"
        case .bottom: "bottom"
        }
    }
}

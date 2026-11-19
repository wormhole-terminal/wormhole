// text-file: a developer tool's note (inspect mode), English like the GPUI inspector's; never interface text
// Nudging in inspect mode (`inspector/edit.rs`'s steppers), debug builds only. An element's style names scale values
// (`padding x _1 · radius chip · height keyCapHeight`); a part whose value is one of `Space`, `Metric` or `Radius` can be
// stepped: a grid step (4 pt) for spacing and sizes, 2 pt for a radius, never below zero. The step is written to the
// scales' debug table (`DesignSystem.DesignOverrides`), so it shows at once, on every element that uses that value, not
// only the pinned one; the note records it for the pinned element, in GPUI's format: `change: padding x _1 → _2`.
// Text roles are not in the table and are not stepped. A release build has no table: nothing is nudgeable there.

import DesignSystem
import Foundation

@MainActor
enum InspectNudge {
    /// One part of a style that names a scale value: `padding x` is `_1`, kept in the table as `space._1`.
    struct Part: Equatable {
        let property: String
        let name: String
        let key: String
    }

    /// The parts of `style` that can be stepped.
    static func parts(of style: String) -> [Part] {
        style.components(separatedBy: " · ").compactMap { text in
            let words = text.split(separator: " ").map(String.init)
            guard words.count >= 2, let name = words.last else { return nil }
            let property = words.dropLast().joined(separator: " ")
            let scale = property.hasPrefix("radius") ? "radius" : name.hasPrefix("_") ? "space" : "metric"
            let part = Part(property: property, name: name, key: "\(scale).\(name)")
            return standing(of: part) == nil ? nil : part
        }
    }

    /// The part's value as the panel shows it: its own name, or what it is nudged to.
    static func shown(_ part: Part) -> String {
        guard let nudged = standing(of: part)?.nudged else { return part.name }
        return "\(part.name) → \(name(of: nudged, in: part))"
    }

    /// One step up or down (`direction` 1 or -1).
    static func step(_ part: Part, by direction: CGFloat) {
        #if DEBUG
            guard let state = standing(of: part) else { return }
            let size: CGFloat = part.key.hasPrefix("radius.") ? 2 : 4
            DesignOverrides.shared.set(part.key, to: max(0, (state.nudged ?? state.standard) + size * direction))
        #endif
    }

    /// `padding x _1 → _2` for every nudged part of `style`.
    static func changes(of style: String) -> [String] {
        parts(of: style).compactMap { part in
            standing(of: part)?.nudged.map { "\(part.property) \(part.name) → \(name(of: $0, in: part))" }
        }
    }

    /// Every value back to its standard (the panel's Reset).
    static func reset() {
        #if DEBUG
            DesignOverrides.shared.reset()
        #endif
    }

    /// A spacing value by its grid step (`_2`, as GPUI names any whole step), anything else in pixels.
    private static func name(of value: CGFloat, in part: Part) -> String {
        let steps = value / 4
        if part.key.hasPrefix("space."), steps == steps.rounded() { return String(format: "_%.0f", steps) }
        return String(format: "%gpx", value)
    }

    private static func standing(of part: Part) -> (standard: CGFloat, nudged: CGFloat?)? {
        #if DEBUG
            return DesignOverrides.shared.state(of: part.key)
        #else
            return nil
        #endif
    }
}

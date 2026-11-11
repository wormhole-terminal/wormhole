// A family as a SwiftUI font. A name is drawn in its own face unless the family is a symbol font or CoreText has
// no such family (a custom font whose file is gone), when the system font stands in; the terminal's own sample
// then stands in with the system's monospaced one so it still reads as a terminal.

import AppKit
import CoreText
import SwiftUI

public enum FontFace {
    /// `SYSTEM_FONT`: what core's font settings call the system UI font.
    public static let system = ".SystemUIFont"

    public static func font(_ family: String, size: CGFloat, ligatures: Bool = true, monospacedFallback: Bool = false) -> Font {
        guard !family.isEmpty, family != system, let font = CoreTextFonts.font(named: family, size: size) else {
            return .system(size: size, design: monospacedFallback ? .monospaced : .default)
        }
        guard !ligatures else { return Font(font) }
        // As the terminal does (`ligature_features`): `calt` and `liga` off.
        let off = CTFontDescriptorCreateWithAttributes(
            [
                kCTFontFeatureSettingsAttribute: ["calt", "liga"].map {
                    [kCTFontOpenTypeFeatureTag: $0, kCTFontOpenTypeFeatureValue: 0] as [CFString: Any]
                }
            ] as CFDictionary)
        return Font(CTFontCreateCopyWithAttributes(font, size, nil, off))
    }
}

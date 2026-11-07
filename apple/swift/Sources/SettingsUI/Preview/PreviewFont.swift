// The font a scene sets its terminal text in: the family the settings name, or a monospaced system face while
// that family is not installed on this Mac (the scene must read as a terminal either way).

import AppKit
import SwiftUI

enum PreviewFont {
    static func mono(_ family: String, size: CGFloat) -> Font {
        NSFont(name: family, size: size) != nil || NSFontManager.shared.availableMembers(ofFontFamily: family) != nil
            ? .custom(family, size: size) : .system(size: size, design: .monospaced)
    }
}

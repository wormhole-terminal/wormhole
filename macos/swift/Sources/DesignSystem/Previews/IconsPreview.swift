// Xcode's canvas for the icons. A preview has a file of its own: the canvas rewrites the file a preview is in, and
// that rewrite does not compile an initializer that hands on to another in one expression (`Image.init(_:)` in Icon.swift).

import SwiftUI

#if DEBUG
#Preview("Icons") {
    LazyVGrid(columns: Array(repeating: GridItem(.fixed(28)), count: 8), spacing: 12) {
        ForEach(Icon.allCases, id: \.self) { Image($0).font(.system(size: 16)) }
    }
    .padding(16)
}
#endif

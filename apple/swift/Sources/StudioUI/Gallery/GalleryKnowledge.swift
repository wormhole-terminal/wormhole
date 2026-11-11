// What the Studio knows about tokens: core's rules (the generated table) and, from the gallery's own specimens,
// who draws with what. The two halves of `TokenKnowledge`, so the FFI can replace the first without the second.

import DesignSystem
import StudioModel

public struct GalleryKnowledge: TokenKnowledge {
    private let rules = CoreKnowledge()

    public init() {}

    public func rule(for token: String, appearance: ThemeAppearance) -> String? { rules.rule(for: token, appearance: appearance) }

    public var uses: [TokenUse] { GalleryUses.elements }
}

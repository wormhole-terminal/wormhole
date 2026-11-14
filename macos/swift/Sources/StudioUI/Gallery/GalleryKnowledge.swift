// What the Studio knows about tokens: core's rules (the generated table), core's `TOKEN_USES` handed in by whoever has
// the core (what "Used by" and "n uses" tell, as in the GPUI Studio), and, from the gallery's own specimens, what the
// views really draw with, which only the gallery's outlines follow. Without the core the gallery's table stands for both.

import DesignSystem
import StudioModel

public struct GalleryKnowledge: TokenKnowledge {
    private let rules = CoreKnowledge()
    private let told: [TokenUse]?

    /// `uses`: core's `TOKEN_USES` (`studio_token_uses`).
    public init(uses: [TokenUse]? = nil) { told = uses }

    public func rule(for token: String, appearance: ThemeAppearance) -> String? { rules.rule(for: token, appearance: appearance) }

    public var uses: [TokenUse] { told ?? GalleryUses.elements }

    public var drawn: [TokenUse] { GalleryUses.elements }
}

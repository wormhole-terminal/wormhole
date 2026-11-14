// Core's `TOKEN_USES` over the bridge (`studio_token_uses`): the components of the design and what they draw with, which
// the Studio's "Used by", a metric's "n uses" and the token list's area filter tell, as the GPUI Studio does.

import StudioModel
import WormholeBindings

public enum FfiTokenUses {
    /// Every use, in core's order; one whose area this Studio does not know is left out.
    public static func all() -> [TokenUse] {
        studioTokenUses().compactMap { use in
            StudioArea(rawValue: use.area).map { TokenUse(id: use.id, label: use.label, area: $0, tokens: use.tokens) }
        }
    }
}

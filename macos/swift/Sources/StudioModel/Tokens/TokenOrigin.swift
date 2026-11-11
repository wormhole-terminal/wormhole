// Where a token's value comes from, as the list says it: the theme sets it itself, it still has the value it was
// copied with (a theme made from another carries a `base`), or Wormhole works it out from other tokens and its rules.

public enum TokenOrigin: Sendable, Equatable {
    case yours, base, workedOut
}

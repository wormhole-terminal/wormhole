// Where a token's value comes from, as the list says it: the theme sets it itself, or Wormhole works it out
// (from the theme it extends and the rules). The GPUI app has a third, "base", for values copied from another
// theme; a Swift draft extends by name and copies nothing, so there is no such thing to show.

public enum TokenOrigin: Sendable, Equatable {
    case yours, workedOut
}

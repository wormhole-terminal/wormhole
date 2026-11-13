// The font an open picker is looking at, for the page's scene to show (GPUI's specimen follows the pointer in the
// list the same way): a family and the size of the slot it would be set in.

public struct FontPreview: Equatable, Sendable {
    /// The slot being picked (`terminal`, `code`, `interface`): the specimen switches to it.
    public let slot: String
    public let family: String
    public let size: Double
    public init(slot: String, family: String, size: Double) { (self.slot, self.family, self.size) = (slot, family, size) }
}

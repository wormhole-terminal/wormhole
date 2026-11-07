// The font an open picker is looking at, for the page's scene to show (GPUI's specimen follows the pointer in the
// list the same way): a family and the size of the slot it would be set in.

public struct FontPreview: Equatable, Sendable {
    public let family: String
    public let size: Double
    public init(family: String, size: Double) { (self.family, self.size) = (family, size) }
}

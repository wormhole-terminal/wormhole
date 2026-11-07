// What the open list is made of, in order: the choice that follows another font, the captions over groups, and
// the families. A value, so the list is a plain function of the catalog and the query.

public struct FontChoice: Sendable, Equatable, Hashable {
    public let source: FontSource
    public let name: String
    public init(source: FontSource, name: String) { (self.source, self.name) = (source, name) }
}

public enum FontEntry: Identifiable, Equatable, Sendable {
    public enum Caption: Equatable, Sendable { case recent, source(FontSource) }

    /// The empty family: "System font" (interface) or "Same as terminal" (code).
    case inherit(String)
    case caption(Caption)
    /// `group` tells a recent family from the same one in its source group; they are two rows.
    case family(FontChoice, group: Caption?)

    public var id: String {
        switch self {
        case .inherit: "inherit"
        case .caption(let caption): "caption-\(caption)"
        case .family(let choice, let group): "family-\(group.map { "\($0)" } ?? "list")-\(choice.name)"
        }
    }

    /// Captions are headings: the keyboard and the pointer pass over them.
    public var isSelectable: Bool {
        if case .caption = self { return false }
        return true
    }

    /// The family value this row sets: empty for the inherit row, nil for a caption.
    public var family: String? {
        switch self {
        case .inherit: ""
        case .caption: nil
        case .family(let choice, _): choice.name
        }
    }
}

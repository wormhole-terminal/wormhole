// The one way text is cut. A text is shown whole whenever the room it is given holds it; the layout decides from the
// real width, never a count of characters or a width fixed in advance. When it does not fit, the end that loses least
// is cut (a name at its end, a path at its front, an identifier whose two ends both matter in its middle), and the
// whole text is the tooltip then. VoiceOver reads a `Text` whole whatever is cut, so the label needs nothing more.
// A sentence is not cut at all: it wraps (`Text` with no line limit).

import SwiftUI

/// Where a cut text loses its characters.
public enum CutEnd: Sendable {
    /// A name or a title: its first letters tell it apart.
    case end
    /// A path or a folder: its last part tells it apart (`…/services/gateway`).
    case start
    /// An identifier whose two ends both matter: a dotted token name, a hash, a file name with its extension.
    case middle

    var mode: Text.TruncationMode {
        switch self {
        case .end: .tail
        case .start: .head
        case .middle: .middle
        }
    }
}

/// One line of `whole`, cut at `end` only when its room is too small, with the whole text as the tooltip then. The font
/// and the colour are the environment's: set them on the `CutText`.
public struct CutText: View {
    private let whole: String
    private let end: CutEnd
    @State private var cut = false

    public init(_ whole: String, at end: CutEnd = .end) { (self.whole, self.end) = (whole, end) }

    public var body: some View {
        Text(whole).lineLimit(1).truncationMode(end.mode)
            .cutProbe(whole, cut: $cut)
            .help(cut ? whole : "")
    }
}

extension View {
    /// Sets `cut` to whether `text` (drawn in `font`, else the environment's) is wider than the room this view is given. A
    /// hidden copy at its whole width is measured against the room, so what is cut off can say so in a tooltip.
    public func cutProbe(_ text: String, font: Font? = nil, cut: Binding<Bool>) -> some View {
        modifier(CutProbe(text: text, font: font, cut: cut))
    }
}

private struct CutProbe: ViewModifier {
    let text: String
    let font: Font?
    @Binding var cut: Bool
    @State private var room: CGFloat = 0
    @State private var whole: CGFloat = 0

    func body(content: Content) -> some View {
        content
            .onGeometryChange(for: CGFloat.self, of: \.size.width) { room = $0; cut = whole > room + 0.5 }
            .background {
                Text(text).font(font).fixedSize().hidden()
                    .onGeometryChange(for: CGFloat.self, of: \.size.width) { whole = $0; cut = whole > room + 0.5 }
            }
    }
}

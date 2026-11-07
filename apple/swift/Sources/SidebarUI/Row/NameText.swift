// A name as a row draws it: whole when it fits, and when it doesn't, cut at its room and faded out over its last 32
// points instead of an ellipsis, so it uses the whole width (`name_text`). Under the pointer, where a × shows at the
// line's end, it is cut shorter so it has faded out before it. The fade is a mask, so the text underneath is whole.
// An invisible, tail-truncating copy of the name takes the room the layout gives it (so the pin after it stays beside
// a short name); a second invisible copy at its full width says whether the name was cut.

import SwiftUI

struct NameText: View {
    let name: String
    let font: Font
    let color: Color
    /// Room the × takes from the line's end while the pointer is over the row.
    let reserve: CGFloat
    @State private var room: CGFloat = 0
    @State private var whole: CGFloat = 0

    /// How far before its line's end a name too long for it fades to nothing (`NAME_FADE`).
    private static let fade: CGFloat = 32

    private var isCut: Bool { whole > room + 0.5 }

    var body: some View {
        Text(name).font(font).lineLimit(1).hidden()
            .padding(.trailing, reserve)
            .onGeometryChange(for: CGFloat.self, of: { $0.size.width - reserve }) { room = $0 }
            .background { Text(name).font(font).fixedSize().hidden().onGeometryChange(for: CGFloat.self, of: \.size.width) { whole = $0 } }
            .overlay(alignment: .leading) {
                Text(name).font(font).foregroundStyle(color).lineLimit(1).fixedSize()
                    .mask(alignment: .leading) { fadeMask }
            }
            .clipped()
            .animation(nil, value: reserve)
            .accessibilityHidden(true)
    }

    @ViewBuilder private var fadeMask: some View {
        if isCut {
            HStack(spacing: 0) {
                Rectangle().frame(width: max(room - Self.fade, 0))
                LinearGradient(colors: [.black, .clear], startPoint: .leading, endPoint: .trailing).frame(width: min(Self.fade, room))
                Spacer(minLength: 0)
            }
        } else {
            Rectangle()
        }
    }
}

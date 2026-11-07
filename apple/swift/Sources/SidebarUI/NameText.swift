// A name as a row draws it: whole when it fits, and when it doesn't, cut at its room and faded out over its last 32
// points instead of an ellipsis, so it uses the whole width (`name_text`). Under the pointer, where a × shows at the
// line's end, it is cut shorter so it has faded out before it. The fade is a mask, so the text underneath is whole.

import SwiftUI

struct NameText: View {
    let name: String
    let font: Font
    let color: Color
    /// Room the × takes from the line's end while the pointer is over the row.
    let reserve: CGFloat

    /// How far before its line's end a name too long for it fades to nothing (`NAME_FADE`).
    private static let fade: CGFloat = 32

    var body: some View {
        let text = Text(name).font(font).foregroundStyle(color).lineLimit(1).fixedSize()
        ViewThatFits(in: .horizontal) {
            text.padding(.trailing, reserve)
            text.frame(maxWidth: .infinity, alignment: .leading)
                .padding(.trailing, reserve)
                .clipped()
                .mask {
                    HStack(spacing: 0) {
                        Rectangle()
                        LinearGradient(colors: [.black, .clear], startPoint: .leading, endPoint: .trailing).frame(width: Self.fade)
                        Color.clear.frame(width: reserve)
                    }
                }
        }
        .animation(nil, value: reserve)
    }
}

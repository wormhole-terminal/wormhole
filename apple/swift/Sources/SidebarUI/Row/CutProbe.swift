// Whether a line of text is wider than the room its view is given, so that what is cut off can say so in a tooltip
// (`row/face.rs` `cut_name`, `cut_detail`). A hidden copy at its whole width is measured against the room.

import SwiftUI

extension View {
    func cutProbe(_ text: String, font: Font, cut: Binding<Bool>) -> some View {
        modifier(CutProbe(text: text, font: font, cut: cut))
    }
}

private struct CutProbe: ViewModifier {
    let text: String
    let font: Font
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

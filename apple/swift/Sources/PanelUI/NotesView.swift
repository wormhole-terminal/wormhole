// The Notes tab: the Next steps card above the Notes card, in the panel's scrolling column.

import PanelModel
import SwiftUI

public struct NotesView: View {
    @Binding var model: NotesModel

    public init(model: Binding<NotesModel>) { _model = model }

    public var body: some View {
        PanelScroll {
            StepsCard(model: $model)
            NotesCard(model: $model)
        }
    }
}

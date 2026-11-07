// The last row of the list: type a step, press return, type the next. Focus stays so a list can be written in
// one go.

import DesignSystem
import SwiftUI

struct AddTaskField: View {
    let add: (String) -> Void
    @State private var text = ""
    @FocusState private var focused: Bool

    var body: some View {
        HStack(spacing: Space._2) {
            Image(systemName: "plus").font(.system(size: 11, weight: .medium)).foregroundStyle(.tertiary)
                .frame(width: 16, height: 16)
            TextField("Add a step", text: $text)
                .textFieldStyle(.plain)
                .font(.callout)
                .focused($focused)
                .onSubmit {
                    add(text)
                    text = ""
                    focused = true
                }
        }
        .padding(.horizontal, Space._3)
        .padding(.vertical, Space._1 + 2)
        .accessibilityElement(children: .contain)
    }
}

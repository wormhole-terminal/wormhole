// The recorder for SwiftUI: the pill from `ShortcutRecorderView`, and what stands in the way of the last key under
// it in the danger colour. The binding changes only with a shortcut that is free.

import DesignSystem
import PlatformModel
import SwiftUI

public struct ShortcutRecorder: View {
    @Binding var shortcut: KeySpec?
    let own: ActionID?
    @State private var message: String?
    @Environment(\.tokens) private var tokens

    public init(_ shortcut: Binding<KeySpec?>, own: ActionID? = nil) {
        _shortcut = shortcut
        self.own = own
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: Space._1) {
            Representable(shortcut: $shortcut, own: own, message: $message, style: .themed(tokens))
                .frame(width: 160, height: Metric.control)
            if let message {
                Text(message)
                    .font(.system(size: TextRole.caption.size))
                    .foregroundStyle(tokens.statusDanger)
                    .accessibilityLabel(Text(message))
            }
        }
    }

    private struct Representable: NSViewRepresentable {
        @Binding var shortcut: KeySpec?
        let own: ActionID?
        @Binding var message: String?
        let style: ShortcutRecorderStyle

        func makeNSView(context: Context) -> ShortcutRecorderView { ShortcutRecorderView() }

        func updateNSView(_ view: ShortcutRecorderView, context: Context) {
            view.shortcut = shortcut
            view.ownAction = own
            view.style = style
            view.onChange = { shortcut = $0; message = nil }
            view.onMessage = { message = $0 }
        }
    }
}

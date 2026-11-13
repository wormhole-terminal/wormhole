// The window's own chrome on sample data, for the Theme Studio's gallery: the status bar, the right panel's icon tabs and a
// column's resize handle with its readout, drawn by the views the window uses. The Studio is a layer below the window and
// cannot name them, so the app hands these to its gallery (`GalleryExtras`).
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem
import PanelUI
import SwiftUI
import WordsModel
import WorkspaceModel

@MainActor
public enum WindowSpecimens {
    /// The specimen `id` names, or nil for one that is not the window's.
    public static func view(for id: String) -> AnyView? {
        switch id {
        case "statusbar": AnyView(SampleWindow { StatusBarSample() })
        case "icontabs": AnyView(SampleWindow { PanelTabBar(tab: .changes, select: { _ in }, badges: [.changes: 3, .usage: 2]) })
        case "resize": AnyView(ResizeSample())
        default: nil
        }
    }
}

/// What the window's views read from their window: its look and its keyboard focus, over the sample workspace. Nothing in
/// it answers a click.
private struct SampleWindow<Content: View>: View {
    @State private var hub = WindowHub(store: WorkspaceStore(backend: MockWorkspaceBackend(selected: 1)), terminals: TerminalViews { _ in nil })
    @State private var look = WindowLook()
    @ViewBuilder var content: Content

    var body: some View {
        content.environment(look).environment(hub.focus).allowsHitTesting(false)
    }
}

/// The bar wide, with its words, and narrow, where the counts keep a glyph (`gallery/sidebar.rs`, `status_bar`).
private struct StatusBarSample: View {
    private static let widths: [CGFloat] = [1_200, 640]

    var body: some View {
        let state = MockWorkspace.state
        VStack(alignment: .leading, spacing: Space._2) {
            ForEach(Self.widths, id: \.self) { width in
                StatusBar(terminals: state.all, selected: state.selectedTerminal, size: "120×40", windowWidth: width) { _ in }
                    .frame(maxWidth: width)
            }
        }
    }
}

/// A column's edge and the readout beside it while it is dragged (`gallery/ambient.rs`): the handle lights when the pointer
/// rests on it and while it is dragged, as in the window, and moves nothing here.
private struct ResizeSample: View {
    @Environment(\.chrome) private var colours

    var body: some View {
        HStack(spacing: Space._3) {
            ColumnHandle(label: Words.text("window-resize-sidebar", fallback: "Resize sidebar"), width: 430, drag: { _ in }, reset: {}, step: { _ in })
                .background { colours.divider.frame(width: colours.dividerWidth) }
                .frame(height: Chrome.hit)
            // The readout stands off the pointer by a step; here it stands in the row.
            ResizeReadout(text: "430 · max", pointer: -Space._2, pointerY: Space._2)
            ResizeReadout(text: "350", pointer: -Space._2, pointerY: Space._2)
            ResizeReadout(text: "Close", pointer: -Space._2, pointerY: Space._2)
        }
    }
}

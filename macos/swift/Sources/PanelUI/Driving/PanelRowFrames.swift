// Where the panel's rows are in the window, for the control folder's right-clicks on them (`changes-menu`, `history-menu`,
// `files-row-menu`): each row on screen records its frame under an id (`file:<path>` for a changed or a commit's file,
// `commit:<id>` for a commit, `details` for the open commit's block, `entry:<path>` for a Files row), in the window
// content's points from its top left (the hosting view is the window's content). A row that goes takes its frame back.
// Settings' rows do the same (`SettingsUI.RowFrames`).

import SwiftUI

@MainActor
public enum PanelRowFrames {
    private static var frames: [String: CGRect] = [:]

    /// The frame recorded under `id`, nil while no such row is on screen.
    public static func frame(_ id: String) -> CGRect? { frames[id] }

    static func set(_ id: String, _ frame: CGRect) { frames[id] = frame }

    /// Only the row that recorded it takes a frame back (a row that replaced it under the same id keeps its own).
    static func remove(_ id: String, at frame: CGRect) { if frames[id] == frame { frames[id] = nil } }
}

extension View {
    /// Records this row's frame under `id` for the control folder.
    func panelRowFrame(_ id: String) -> some View { modifier(PanelRowFrameRecorder(id: id)) }
}

private struct PanelRowFrameRecorder: ViewModifier {
    let id: String
    @State private var recorded: CGRect?

    func body(content: Content) -> some View {
        content
            .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { frame in
                recorded = frame
                PanelRowFrames.set(id, frame)
            }
            .onDisappear { if let recorded { PanelRowFrames.remove(id, at: recorded) } }
    }
}

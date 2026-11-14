// Where each row's control (and its reset arrow) is in the window, for the control folder's `settings-rows`: a driver
// clicks the real control instead of guessing points from a capture. Kept for every row that is laid out; rows leave
// it when their view goes (a page change), so a listing is of the page on show.

import SwiftUI

@MainActor
public enum RowFrames {
    private static var frames: [String: CGRect] = [:]

    /// `id` is the row's id, `id.reset` its reset arrow; the frames are in the window's points from its top left.
    public static var all: [(id: String, frame: CGRect)] {
        frames.map { ($0.key, $0.value) }.sorted { ($0.1.minY, $0.1.minX) < ($1.1.minY, $1.1.minX) }
    }
}

extension View {
    /// Records this view's frame under `id` while it is on show (nil: not recorded).
    func recordFrame(_ id: String?) -> some View { modifier(FrameRecorder(id: id)) }
}

private struct FrameRecorder: ViewModifier {
    let id: String?
    /// The frame this view recorded: what it takes back when it goes.
    @State private var recorded: CGRect?

    func body(content: Content) -> some View {
        content
            .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { frame in
                guard let id else { return }
                recorded = frame
                RowFrames.set(id, frame)
            }
            .onDisappear { if let id, let recorded { RowFrames.remove(id, at: recorded) } }
    }
}

extension RowFrames {
    fileprivate static func set(_ id: String, _ frame: CGRect) { frames[id] = frame }
    /// Only the frame the leaving view recorded: a search result's row goes after the page's row of the same id has come, and
    /// must not take that one's frame with it.
    fileprivate static func remove(_ id: String, at frame: CGRect) { if frames[id] == frame { frames[id] = nil } }
}

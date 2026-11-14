// Where each row's control (and its reset arrow) is in the window, for the control folder's `settings-rows`: a driver
// clicks the real control instead of guessing points from a capture. Kept for every row that is laid out; rows leave
// it when their view goes (a page change), so a listing is of the page on show.

import SwiftUI

@MainActor
public enum RowFrames {
    /// Each frame with the view that recorded it.
    private static var frames: [String: (frame: CGRect, owner: ObjectIdentifier)] = [:]

    /// `id` is the row's id, `id.reset` its reset arrow; the frames are in the window's points from its top left.
    public static var all: [(id: String, frame: CGRect)] {
        frames.map { ($0.key, $0.value.frame) }.sorted { ($0.1.minY, $0.1.minX) < ($1.1.minY, $1.1.minX) }
    }
}

extension View {
    /// Records this view's frame under `id` while it is on show (nil: not recorded).
    func recordFrame(_ id: String?) -> some View { modifier(FrameRecorder(id: id)) }
}

private struct FrameRecorder: ViewModifier {
    let id: String?
    /// This view among the ones that record under the same id, and the frame it last had.
    @State private var held = Held()

    private final class Held {
        var frame: CGRect?
    }

    func body(content: Content) -> some View {
        content
            .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { frame in
                guard let id else { return }
                held.frame = frame
                RowFrames.set(id, frame, by: ObjectIdentifier(held))
            }
            // A view that went and came back where it was is told of no new geometry: it records what it had again.
            .onAppear { if let id, let frame = held.frame { RowFrames.set(id, frame, by: ObjectIdentifier(held)) } }
            .onDisappear { if let id { RowFrames.remove(id, by: ObjectIdentifier(held)) } }
    }
}

extension RowFrames {
    fileprivate static func set(_ id: String, _ frame: CGRect, by owner: ObjectIdentifier) { frames[id] = (frame, owner) }
    /// Only what the leaving view recorded itself: a view of the same id that came before this one went (a search result's
    /// row, a shelf's card built again in the same place) keeps its frame. Told apart by the view, not by the frame: a card
    /// built again stands exactly where the old one stood, and the old one could take the frame with it.
    fileprivate static func remove(_ id: String, by owner: ObjectIdentifier) { if frames[id]?.owner == owner { frames[id] = nil } }
}

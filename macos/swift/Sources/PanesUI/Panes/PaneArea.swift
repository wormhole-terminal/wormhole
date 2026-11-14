// A split drawn: each pane with its header, the dividers over their shared edges, the drop overlay, the whole surface. The
// layout is the core's (`PanesState`: where each pane is, which has the keyboard, which is zoomed); the terminals are the
// window's, handed in through `content`. A zoomed pane fills the area and the others fade but stay alive underneath, so their
// terminals keep their place and the zoom reverses in place. Frames animate when the layout changes (a split opens, a drag
// lands), a fade under Reduce Motion.

import DesignSystem
import PanesModel
import SwiftUI
import WorkspaceModel

enum PaneSpace { static let name = "pane-area" }

public struct PaneArea<Content: View>: View {
    let panes: PanesState
    let gaps: Bool
    let info: (PaneID) -> PaneInfo
    let actions: PaneActions
    let session: PaneDragSession
    /// A terminal dragged out of the sidebar's list, which the list reports as it goes (the list's drag is its own gesture).
    let sidebarDrag: TerminalDragPointer?
    @ViewBuilder let content: (PaneID) -> Content
    /// The Motion feature is on in this window: a split opens with the core's spring only then (and not under Reduce Motion).
    let playsMotion: Bool
    @State var drop = PaneDropState()
    @State private var tracker = SplitOpeningTracker()
    /// Bumped when an opening's time is up, so the body draws the layout as it is.
    @State private var settled = 0
    @Environment(\.reduceMotion) private var reduceMotion

    public init(
        panes: PanesState, gaps: Bool, session: PaneDragSession, sidebarDrag: TerminalDragPointer? = nil,
        playsMotion: Bool = false, actions: PaneActions = PaneActions(), info: @escaping (PaneID) -> PaneInfo, @ViewBuilder content: @escaping (PaneID) -> Content
    ) {
        (self.panes, self.gaps, self.session, self.sidebarDrag, self.actions, self.info, self.content) =
            (panes, gaps, session, sidebarDrag, actions, info, content)
        self.playsMotion = playsMotion
    }

    public var body: some View {
        GeometryReader { proxy in
            let size = proxy.size
            let origin = proxy.frame(in: .global).origin
            let change = tracker.observe(panes, motion: actions.splitMotion, plays: playsMotion && !reduceMotion, split: actions.splitCame())
            let playing = stillPlaying(change)
            TimelineView(.animation(paused: playing == nil)) { timeline in
                let moment = playing.flatMap { now in
                    actions.splitMotion.map { ($0.frame(of: now.opening, elapsed: timeline.date.timeIntervalSince(now.began)), now) }
                }
                ZStack(alignment: .topLeading) {
                    ForEach(panes.unzoomed, id: \.id) { pane in
                        frame(pane.id, at: placement(of: pane, size: size, moment: moment), size: size)
                    }
                    // The lines of a layout that is not there yet wait for the motion to end.
                    PaneDividers(dividers: playing == nil ? panes.dividers : [], gaps: gaps, session: session, actions: actions)
                    PaneDropZones(panes: panes, state: drop, size: size, names: { info($0).title }, refusal: actions.refusal)
                    if let over = sidebarDrag?.over { ghost(over, origin: origin) }
                }
            }
            .frame(width: size.width, height: size.height, alignment: .topLeading)
            .coordinateSpace(.named(PaneSpace.name))
            .onDrop(of: [.plainText], delegate: PaneDropDelegate(state: $drop, panes: panes, actions: actions))
            // A split that opens is drawn frame by frame below; one that is cut (Reduce Motion, Motion off) lands at once.
            .animation(playing != nil || change.isCut ? nil : (reduceMotion ? Motion.fade : Motion.layout), value: panes)
            .task(id: playing?.began) {
                guard let playing else { return }
                // one-shot: the opening's end, when the panes are laid out for good
                try? await Task.sleep(for: .seconds(max(0, playing.opening.duration - Date().timeIntervalSince(playing.began)) + 0.05))
                if !Task.isCancelled { settled &+= 1 }
            }
            .onChange(of: size, initial: true) { _, size in actions.areaChanged(size) }
            .onChange(of: sidebarDrag?.over, initial: true) { _, over in follow(over, origin: origin) }
            .onChange(of: sidebarDrag?.released) { _, released in if let released { land(released, origin: origin) } }
        }
    }

    /// The opening still moving: the one the layout came by, until its time is up.
    private func stillPlaying(_ change: SplitOpeningTracker.Change) -> SplitOpeningTracker.Playing? {
        _ = settled
        guard case .plays(let now) = change, !now.ended(at: Date()) else { return nil }
        return now
    }

    /// Where a pane's box is and how much of it shows. They are one rectangle but while a split opens: the pane that was split
    /// keeps the box (so the size of its terminal) it had and shows what the edge leaves it, and the new pane has its final
    /// box, moved in over the room of the other, which clips it. No terminal is sized by the motion.
    private struct Placement {
        var box: PaneRect
        var clip: PaneRect

        init(_ rect: PaneRect) { (box, clip) = (rect, rect) }
        init(box: PaneRect, clip: PaneRect) { (self.box, self.clip) = (box, clip) }
    }

    private func placement(of pane: PanePlace, size: CGSize, moment: (SplitFrame, SplitOpeningTracker.Playing)?) -> Placement {
        if panes.zoomed == pane.id { return Placement(PaneRect(0, 0, size.width, size.height)) }
        guard let (frame, playing) = moment else { return Placement(inset(pane.rect)) }
        let opening = playing.opening
        if pane.id == opening.target { return Placement(box: inset(opening.whole), clip: inset(frame.kept)) }
        guard pane.id == playing.new else { return Placement(inset(pane.rect)) }
        let own = inset(pane.rect)
        let moved = PaneRect(
            own.x + frame.newPane.x - opening.newPane.x, own.y + frame.newPane.y - opening.newPane.y, own.w, own.h)
        return Placement(box: moved, clip: opening.whole)
    }

    private func frame(_ id: PaneID, at placement: Placement, size: CGSize) -> some View {
        let split = panes.isSplit
        let zoomed = panes.zoomed
        let shown = zoomed == nil || zoomed == id
        let order = panes.order
        let focused = panes.focused == id
        let dragged = DraggedTerminal(id: id, origin: .paneHeader)
        return PaneFrame(
            focused: focused || !split, gaps: gaps && split,
            lifted: drop.isHovering && drop.dragged == dragged
        ) {
            if split {
                PaneHeader(
                    info: info(id), focused: focused, gaps: gaps, zoomed: zoomed == id, dragged: dragged,
                    position: (order.firstIndex(of: id) ?? 0) + 1, of: order.count,
                    focus: { actions.focus(id) }, zoom: { toggleZoom(id) }, close: { actions.close(id) },
                    tile: actions.tile?(id, PaneMetrics.tile), zoomKey: actions.keyText("zoomPane"), closeKey: actions.keyText("closeTerminal")
                ) {
                    PaneMenuItems(id: id, panes: panes, actions: actions)
                }
            }
        } content: {
            content(id)
        }
        .held(placement.box, visibleIn: placement.clip)
        .opacity(shown ? 1 : 0)
        .allowsHitTesting(shown)
        .accessibilityHidden(!shown)
        .zIndex(zoomed == id ? 1 : 0)
        .motionTransition(.opacity.combined(with: .scale(scale: 0.98)))
    }

    private func toggleZoom(_ id: PaneID) {
        if panes.focused != id { actions.focus(id) }
        actions.toggleZoom()
    }

    /// With gaps a pane is its own box: its cell less the gap on each side that has a neighbour (the gap
    /// belongs to the later pane of a pair, as GPUI's cells pad their start).
    private func inset(_ rect: PaneRect) -> PaneRect {
        guard gaps, panes.isSplit else { return rect }
        let (left, top) = (rect.x > 0.5 ? PaneMetrics.gap : 0, rect.y > 0.5 ? PaneMetrics.gap : 0)
        return PaneRect(rect.x + left, rect.y + top, rect.w - left, rect.h - top)
    }
}

private extension View {
    /// A box of its own size, of which only `clip` shows (both in the area's points): the same rectangle where the two are one.
    func held(_ box: PaneRect, visibleIn clip: PaneRect) -> some View {
        frame(width: max(0, box.w), height: max(0, box.h))
            .offset(x: box.x - clip.x, y: box.y - clip.y)
            .frame(width: max(0, clip.w), height: max(0, clip.h), alignment: .topLeading)
            .clipped()
            .offset(x: clip.x, y: clip.y)
    }
}

// The minimap along the pane's right edge (View → Show Minimap, `render/minimap.rs`): the history and the screen
// drawn small, one block per run of text in its colour, with the part on screen marked in the scrollbar thumb's
// colour. A click on the mark grabs it; anywhere else brings that line to the middle of the screen and grabs the mark
// where it lands; a drag scrolls like a scrollbar. Drawn in a Canvas, repainted only when the snapshot changes.
// VoiceOver gets a slider (the scrollbar's role), moved a screenful at a time.

import DesignSystem
import SwiftUI
import TerminalCore

public struct MinimapView: View {
    let source: MinimapSource
    /// Bumped by the host whenever the terminal printed or scrolled, so the snapshot is read again.
    let revision: Int
    @Environment(\.tokens) private var tokens
    @State private var grab: CGFloat?

    public init(source: MinimapSource, revision: Int) { (self.source, self.revision) = (source, revision) }

    public var body: some View {
        GeometryReader { proxy in
            let fits = max(Int(proxy.size.height / MinimapGeometry.linePitch), 1)
            let snapshot = source.snapshot(fits: fits)
            Canvas { context, _ in
                draw(snapshot, in: &context)
            }
            .gesture(drag(snapshot.geometry))
            .accessibilityElement()
            .accessibilityLabel(L10n.minimap)
            .accessibilityValue(L10n.minimapValue(snapshot.geometry.viewTop, snapshot.geometry.total))
            .accessibilityAdjustableAction { direction in
                let g = snapshot.geometry
                let page = direction == .increment ? g.screen : -g.screen
                scroll(g, viewTop: min(max(g.viewTop + page, 0), max(g.total - g.screen, 0)))
            }
        }
        .frame(width: MinimapGeometry.width)
    }

    private func draw(_ snapshot: MinimapSnapshot, in context: inout GraphicsContext) {
        let column = min(MinimapGeometry.width / CGFloat(max(snapshot.columns, 1)), MinimapGeometry.columnMax)
        for (k, runs) in snapshot.lines.enumerated() {
            let y = MinimapGeometry.linePitch * CGFloat(k)
            for run in runs {
                let rect = CGRect(x: column * CGFloat(run.start), y: y, width: column * CGFloat(run.end - run.start),
                                  height: MinimapGeometry.blockHeight)
                context.fill(Path(rect), with: .color(color(run)))
            }
        }
        guard snapshot.geometry.scrolls else { return }
        let slider = snapshot.geometry.slider
        context.fill(Path(CGRect(x: 0, y: slider.top, width: MinimapGeometry.width, height: slider.height)),
                     with: .color(tokens.scrollbarThumb))
    }

    private func color(_ run: MinimapRun) -> Color {
        let cell = run.cell.cellFlags
        var (fg, bg) = (source.palette.resolve(run.cell.fg), source.palette.resolve(run.cell.bg))
        if cell.contains(.inverse) { swap(&fg, &bg) }
        // Dim text is the foreground at two thirds over the background (`MetalRenderer.dim`).
        if run.isText, cell.contains(.dim) { fg = Self.blend(fg, over: bg, 0.66) }
        let c = run.isText ? fg : bg
        return Color(.sRGB, red: Double(c.x) / 255, green: Double(c.y) / 255, blue: Double(c.z) / 255)
    }

    private static func blend(_ a: SIMD3<UInt8>, over b: SIMD3<UInt8>, _ t: Double) -> SIMD3<UInt8> {
        func mix(_ x: UInt8, _ y: UInt8) -> UInt8 { UInt8(Double(x) * t + Double(y) * (1 - t)) }
        return SIMD3(mix(a.x, b.x), mix(a.y, b.y), mix(a.z, b.z))
    }

    private func drag(_ g: MinimapGeometry) -> some Gesture {
        DragGesture(minimumDistance: 0, coordinateSpace: .local)
            .onChanged { value in
                if grab == nil {
                    let slider = g.slider
                    if (slider.top..<slider.top + slider.height).contains(value.startLocation.y) {
                        grab = value.startLocation.y - slider.top
                    } else {
                        let line = g.line(atY: value.startLocation.y) ?? max(g.total - 1, 0)
                        let top = g.viewTop(centering: line)
                        scroll(g, viewTop: top)
                        grab = value.startLocation.y - MinimapGeometry(total: g.total, screen: g.screen, fits: g.fits, viewTop: top).slider.top
                    }
                }
                if let grab { scroll(g, viewTop: g.viewTop(forSliderY: value.location.y - grab)) }
            }
            .onEnded { _ in grab = nil }
    }

    private func scroll(_ g: MinimapGeometry, viewTop: Int) { source.scroll(toDisplayOffset: g.displayOffset(viewTop: viewTop)) }
}

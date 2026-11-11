// Text zoom (⌘+ ⌘- ⌘0) and live font settings. The zoom is points over the size in Settings, per view, not saved
// (`window_actions::zoomed`); the size eases there over 120 ms in quarter points, cut under Reduce Motion
// (`TextZoomEase`, core `view::motion::text_zoom`). While it moves the program keeps the grid it had (the drawn rows
// are clipped, or leave blank edges, anchored top left as in GPUI) and is told the new one once, on arrival.

import AppKit
import TerminalCore

extension MetalTerminalView {
    /// The fonts as Settings has them, before the zoom.
    public var fontSettings: TerminalFontSettings { baseFonts }

    /// The size the text is drawn at when it is not moving: Settings' size plus the zoom, within 8...36.
    public var pointSize: Double { baseFonts.at(pointSize: baseFonts.pointSize + zoom).pointSize }

    /// Settings changed the family, size, line height or ligatures: draw with them at once (a settings change is a
    /// cut; only the zoom eases), keeping the zoom.
    public func setFonts(_ settings: TerminalFontSettings) {
        baseFonts = settings
        zoomEase = nil
        applyFonts(at: pointSize)
        settleGrid()
    }

    /// Draw the text this many points over the size in Settings. `animated: false` adopts a zoom without easing
    /// or saying anything (a terminal new to a window that is zoomed).
    public func setZoom(_ points: Double, animated: Bool = true) {
        guard points != zoom else { return }
        zoom = points
        let target = pointSize
        guard animated, animatesZoom, !reduceMotion, window != nil else {
            zoomEase = nil
            applyFonts(at: target)
            settleGrid(announce: animated)
            return
        }
        let now = Date()
        zoomEase = zoomEase?.retargeted(at: now, to: target) ?? TextZoomEase(from: renderer.configuration.fonts.pointSize, to: target, began: now)
        requestRedraw()
    }

    /// ⌘+ (`step` 1) and ⌘- (`step` -1): a point bigger or smaller, within what Settings allows.
    public func zoom(step: Double) {
        setZoom(TextZoom.zoomed(base: baseFonts.pointSize, zoom: zoom, step: step))
    }

    /// ⌘0: the size in Settings.
    public func resetZoom() { setZoom(0) }

    /// A text size in points (the zoom that gives it), for a host that holds sizes rather than zooms.
    public func setPointSize(_ points: Double, animated: Bool = true) {
        setZoom(min(max(points, TextZoom.sizes.lowerBound), TextZoom.sizes.upperBound) - baseFonts.pointSize, animated: animated)
    }

    /// One frame of the ease: the size it has reached; on arrival the program is told the grid.
    func advanceZoom(at now: Date = Date()) {
        guard let ease = zoomEase else { return }
        let sample = ease.at(now)
        applyFonts(at: sample.size)
        if sample.done {
            zoomEase = nil
            settleGrid(announce: true)
        } else {
            requestRedraw()
        }
    }

    private func applyFonts(at size: Double) {
        renderer.setFonts(baseFonts.at(pointSize: size))
        requestRedraw()
    }

    /// The cell size is final: the grid the view fits is told to the program, and the host may say it.
    private func settleGrid(announce: Bool = false) {
        syncSurface()
        if announce { onZoomSettled?(renderer.gridSize.cols, renderer.gridSize.rows) }
    }
}

// xterm's window operations carried out on the terminal's window, and the window's place told to the engine for the
// reports a program may ask for (the GPUI app's `window_ops.rs` and `terminal_view/window_geometry.rs`). What a move or a
// resize comes to is the core's sum (`window_op_frame`, `window_geometry`, in points from the top left of the window's
// screen); this turns AppKit's frames round and does what AppKit does: miniaturise, order, zoom, full screen.

#if WITH_UNIFFI
    import AppKit
    import TerminalCore
    import TerminalMetal
    import UniFFISource
    import WormholeBindings

    /// why: the view is only read on the main actor (the task above hops there); the box only carries it across.
    private final class WeakView: @unchecked Sendable {
        weak var view: MetalTerminalView?
        init(_ view: MetalTerminalView) { self.view = view }
    }

    @MainActor
    enum WindowOps {
        /// Sizes windows had before a maximise, to go back to (XTWINOPS 9;0).
        private static var saved: [ObjectIdentifier: NSSize] = [:]
        /// Views whose program asked for a size the window has not followed yet: the engine already has it, and the
        /// window's would undo that.
        private static var pending: Set<ObjectIdentifier> = []
        private static var watching = false
        /// The size a program's last resize asked of the window, until the layout shows it or a second passes: the next
        /// operation, which a program sends at once, is about that size (`size_asked`). With the grid it was asked at.
        private static var asked: [ObjectIdentifier: (cols: Int, rows: Int, before: GridSize, at: ContinuousClock.Instant)] = [:]

        /// `view`'s program may act on its window (when `allow_window_ops` is on) and is told where it is. `raises`: a
        /// raise may bring the window forward (not in a test instance, which stays behind).
        static func attach(_ view: MetalTerminalView, raises: Bool) {
            guard let source = view.source as? UniFFIFrameSource else { return }
            let target = WeakView(view)
            source.onWindowOp { op in
                Task { @MainActor in if let view = target.view { carry(op, for: view, raises: raises) } }
            }
            view.onGridChange = { [weak view] in if let view { share(view) } }
            watchWindows()
        }

        /// Tell the engine of `view`'s terminal where its window is (skipped while a size it asked for is pending).
        static func share(_ view: MetalTerminalView, iconified: Bool? = nil) {
            guard !pending.contains(ObjectIdentifier(view)), let source = view.source as? UniFFIFrameSource,
                let window = view.window, let place = place(of: window)
            else { return }
            let pane = pane(of: view, in: window)
            source.setWindow(windowGeometry(place: place, pane: pane, iconified: iconified ?? window.isMiniaturized))
        }

        private static func carry(_ op: WindowOpRecord, for view: MetalTerminalView, raises: Bool) {
            guard let window = view.window, let place = place(of: window), let screen = window.screen else { return }
            let pane = pane(of: view, in: window)
            switch op {
            case .iconify:
                window.miniaturize(nil)
                share(view, iconified: true)
            case .deiconify:
                window.deminiaturize(nil)
                share(view, iconified: false)
            case .raise: if raises { window.orderFront(nil) }
            case .lower: window.orderBack(nil)
            case .maximize(how: .both):
                asked[ObjectIdentifier(view)] = nil
                if !window.isZoomed {
                    saved[ObjectIdentifier(window)] = saved[ObjectIdentifier(window)] ?? window.frame.size
                    window.zoom(nil)
                }
            case .maximize(how: .restore):
                asked[ObjectIdentifier(view)] = nil
                if window.isZoomed {
                    window.zoom(nil)
                } else if let size = saved[ObjectIdentifier(window)] {
                    window.setFrame(NSRect(origin: window.frame.origin, size: size), display: true)
                }
                saved[ObjectIdentifier(window)] = nil
            case .fullscreen(let how):
                asked[ObjectIdentifier(view)] = nil
                let on = window.styleMask.contains(.fullScreen)
                let want = switch how {
                case .enter: true
                case .exit: false
                case .toggle: !on
                }
                if want != on { window.toggleFullScreen(nil) }
            case .move, .resizeCells, .maximize:
                if case .maximize = op { saved[ObjectIdentifier(window)] = saved[ObjectIdentifier(window)] ?? window.frame.size }
                guard let goal = windowOpFrame(op: op, place: place, pane: pane) else { break }
                if let cols = goal.cols, let rows = goal.rows {
                    asked[ObjectIdentifier(view)] = (Int(cols), Int(rows), view.gridSize, .now)
                } else {
                    asked[ObjectIdentifier(view)] = nil
                }
                window.setFrame(appKitFrame(goal.frame, on: screen), display: true)
            }
            followed(view)
        }

        /// The window takes a moment to follow a resize, and may not at all: then the engine is brought back to the
        /// pane's size and told where the window is.
        private static func followed(_ view: MetalTerminalView) {
            let id = ObjectIdentifier(view)
            pending.insert(id)
            Task { @MainActor [weak view] in
                // one-shot: the window has had 400 ms to follow (`window_op_done` in the GPUI app).
                try? await Task.sleep(for: .milliseconds(400))
                pending.remove(id)
                guard let view else { return }
                (view.source as? UniFFIFrameSource)?.syncSize()
                share(view)
            }
        }

        /// The window, its screen and the part of it a window can use, in points from the screen's top left.
        private static func place(of window: NSWindow) -> WindowPlace? {
            guard let screen = window.screen else { return nil }
            let full = screen.frame
            func local(_ rect: NSRect) -> ScreenRect {
                ScreenRect(
                    x: Float(rect.minX - full.minX), y: Float(full.maxY - rect.maxY), width: Float(rect.width),
                    height: Float(rect.height))
            }
            return WindowPlace(
                window: local(window.frame), screen: local(full), usable: local(screen.visibleFrame),
                fullscreen: window.styleMask.contains(.fullScreen))
        }

        private static func appKitFrame(_ rect: ScreenRect, on screen: NSScreen) -> NSRect {
            let full = screen.frame
            return NSRect(
                x: full.minX + CGFloat(rect.x), y: full.maxY - CGFloat(rect.y) - CGFloat(rect.height),
                width: CGFloat(rect.width), height: CGFloat(rect.height))
        }

        /// The pane's cell, its text area in cells, and what is around the text area in the window.
        private static func pane(of view: MetalTerminalView, in window: NSWindow) -> PaneSizeRecord {
            let grid = view.gridSize
            let text = (width: Float(grid.cols) * grid.cellWidth, height: Float(grid.rows) * grid.cellHeight)
            // What the last resize asked for stands while the layout has not caught up with it (`pane_size`).
            var (cols, rows) = (grid.cols, grid.rows)
            if let ask = asked[ObjectIdentifier(view)], ask.before == grid, ask.at.duration(to: .now) < .seconds(1) {
                (cols, rows) = (ask.cols, ask.rows)
            }
            return PaneSizeRecord(
                cellWidth: grid.cellWidth, cellHeight: grid.cellHeight, cols: UInt32(cols), rows: UInt32(rows),
                frameWidth: max(0, Float(window.frame.width) - text.width), frameHeight: max(0, Float(window.frame.height) - text.height))
        }

        /// A window that moves, changes size, goes to the Dock or full screen tells its terminals.
        private static func watchWindows() {
            guard !watching else { return }
            watching = true
            let names: [Notification.Name] = [
                NSWindow.didMoveNotification, NSWindow.didResizeNotification, NSWindow.didMiniaturizeNotification,
                NSWindow.didDeminiaturizeNotification, NSWindow.didEnterFullScreenNotification,
                NSWindow.didExitFullScreenNotification, NSWindow.didChangeScreenNotification,
            ]
            for name in names {
                NotificationCenter.default.addObserver(forName: name, object: nil, queue: .main) { note in
                    let window = note.object as? NSWindow
                    MainActor.assumeIsolated { shareAll(in: window) }
                }
            }
        }

        /// Every terminal view under `window`'s content.
        private static func shareAll(in window: NSWindow?) {
            guard let content = window?.contentView else { return }
            var stack = [content]
            while let view = stack.popLast() {
                if let terminal = view as? MetalTerminalView { share(terminal) } else { stack.append(contentsOf: view.subviews) }
            }
        }
    }
#endif

// The bell: the system's alert sound, a brief glow along the pane's edge, and a Dock bounce when the app is
// not in front. A program ringing in a loop gets one sound per glow, not a rattle (`terminal_view/status.rs`
// `ring`). Which of the three happen are the settings' (`bell_sound`, `bell_visual`); the host sets `bell`.

import AppKit
import DesignSystem

public struct BellStyle: Sendable, Equatable {
    public var sound: Bool
    public var visual: Bool
    /// The Dock icon bounces once when the bell rings in an app that is not in front (the Mac's convention).
    public var dock: Bool
    public init(sound: Bool = true, visual: Bool = true, dock: Bool = true) {
        self.sound = sound
        self.visual = visual
        self.dock = dock
    }
}

extension MetalTerminalView {
    /// How long the glow stays (`terminal_view/status.rs` `BELL_GLOW`) and how wide its edge is (`overlays.rs` `BELL_EDGE_W`).
    static var bellGlow: DispatchTimeInterval { .milliseconds(150) }
    static var bellEdgeWidth: CGFloat { 2 }

    func ringBell() {
        if bell.sound, !isGlowing, TestInstance.mayMakeSound { NSSound.beep() }
        if bell.dock, !NSApp.isActive { NSApp.requestUserAttention(.informationalRequest) }
        guard bell.visual else { return }
        let glow = glowLayer ?? makeGlowLayer()
        glow.frame = bounds
        glow.borderColor = bellColor.cgColor
        glow.cornerRadius = bellCornerRadius
        glow.isHidden = false
        glowItem?.cancel()
        let item = DispatchWorkItem { [weak self] in
            MainActor.assumeIsolated {
                self?.glowLayer?.isHidden = true
                self?.glowItem = nil
            }
        }
        glowItem = item
        DispatchQueue.main.asyncAfter(deadline: .now() + Self.bellGlow, execute: item)
        requestRedraw()
    }

    private var isGlowing: Bool { glowItem != nil }

    private func makeGlowLayer() -> CALayer {
        let layer = CALayer()
        layer.borderWidth = Self.bellEdgeWidth
        layer.zPosition = 10
        self.layer?.addSublayer(layer)
        glowLayer = layer
        return layer
    }
}

// The system's Colours window for a group's "Other colour…" (`platform/macos/colour.rs`): it opens on the colour the
// group shows and the group follows every step of a drag in it. The panel is the app's one shared panel, so the last
// group asked about is the one it colours; AppKit holds its target weakly, which is why the session is kept here.

import AppKit

@MainActor
final class GroupColourPanel: NSObject {
    private static var session: GroupColourPanel?
    private let change: @MainActor (UInt8, UInt8, UInt8) -> Void

    private init(change: @escaping @MainActor (UInt8, UInt8, UInt8) -> Void) { self.change = change }

    /// Open the Colours window showing `start`; every colour chosen in it reaches `change` as sRGB bytes.
    static func open(start: NSColor, change: @escaping @MainActor (UInt8, UInt8, UInt8) -> Void) {
        let target = GroupColourPanel(change: change)
        let panel = NSColorPanel.shared
        panel.showsAlpha = false
        panel.isContinuous = true
        // The target is wired after the colour is set: setting it is not a choice.
        panel.setTarget(nil)
        panel.color = start
        panel.setTarget(target)
        panel.setAction(#selector(changed(_:)))
        panel.makeKeyAndOrderFront(nil)
        session = target
    }

    @objc private func changed(_ sender: NSColorPanel) {
        guard let rgb = sender.color.usingColorSpace(.sRGB) else { return }
        func byte(_ value: CGFloat) -> UInt8 { UInt8((min(max(value, 0), 1) * 255).rounded()) }
        change(byte(rgb.redComponent), byte(rgb.greenComponent), byte(rgb.blueComponent))
    }
}

extension NSColor {
    /// `#rrggbb`, the form the core keeps a group's own colour in.
    convenience init?(groupHex hex: String) {
        guard hex.hasPrefix("#"), hex.count == 7, let value = UInt32(hex.dropFirst(), radix: 16) else { return nil }
        self.init(srgbRed: CGFloat(value >> 16 & 0xff) / 255, green: CGFloat(value >> 8 & 0xff) / 255, blue: CGFloat(value & 0xff) / 255, alpha: 1)
    }
}

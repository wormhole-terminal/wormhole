import AppKit
import PlatformModel
import Testing

@testable import PlatformUI

/// The wrappers that touch the system: a test instance leaves all of it alone.
@MainActor @Suite(.serialized) struct AppKitEdgeTests {
    static let background = PlatformPolicy(background: true)

    @Test func aTestInstanceRegistersNoHotkey() throws {
        var pressed = 0
        let hotkey = GlobalHotkey(policy: Self.background) { pressed += 1 }
        try hotkey.register(KeySpec([.shift, .command], "space"))
        #expect(hotkey.spec == KeySpec([.shift, .command], "space") && !hotkey.registered)
        #expect(throws: GlobalHotkey.Failure.unknownKey("⌘Nosuch")) { try hotkey.register(KeySpec(.command, "nosuch")) }
        hotkey.unregister()
        #expect(hotkey.spec == nil)
    }

    /// The one real registration, of a chord nobody has (⌃⌥⇧⌘F19), let go at once: Carbon accepts a free chord and
    /// refuses a taken one.
    @Test func carbonAcceptsAFreeChordAndRefusesATakenOne() throws {
        let spec = KeySpec([.control, .option, .shift, .command], "f19")
        let first = GlobalHotkey(policy: PlatformPolicy(background: false)) {}
        let second = GlobalHotkey(policy: PlatformPolicy(background: false)) {}
        try first.register(spec)
        #expect(first.registered)
        #expect(throws: GlobalHotkey.Failure.self) { try second.register(spec) }
        first.unregister()
        try second.register(spec)
        second.unregister()
    }

    @Test func theDockOfATestInstanceIsNotTouched() {
        let dock = DockController(policy: Self.background)
        dock.setBadge("3")
        dock.bounce()
        #expect(dock.badge == "3" && dock.bounces == 1)
        #expect(NSApp.dockTile.badgeLabel == nil, "the real Dock tile was left alone")
        dock.update(terminals: [AmbientTerminal(session: 1, name: "a", state: .needsYou)], badgeMode: .needsYou)
        #expect(dock.badge == "1" && dock.dockMenu().items.map(\.title).contains("New Terminal"))
        dock.update(terminals: [], badgeMode: .needsYou)
        #expect(dock.badge == nil)
    }

    @Test func aTestInstanceHasNoStatusItem() {
        let controller = StatusItemController(policy: Self.background)
        controller.update(StatusSnapshot())
        #expect(controller.item == nil && controller.model?.icon.tooltip == "Wormhole — no terminals open")
    }

    @Test func aTestInstanceNeverPostsABanner() {
        let notifier = SystemNotifier.make(policy: Self.background, bundled: true)
        #expect(notifier is LogNotifier)
        #expect(SystemNotifier.make(policy: PlatformPolicy(background: false), bundled: false) is LogNotifier)
    }

    @Test func theStatusIconDrawsEveryStateOnTheSameCanvas() throws {
        let colors = StatusIconImage.Colors(attention: .orange, done: .green, danger: .red)
        let states: [(Bool, Int?, StatusMark)] = [
            (false, nil, .none), (true, nil, .none), (true, 0, .none), (true, 1, .none), (false, nil, .needsYou), (false, nil, .failed), (false, nil, .finished),
        ]
        var bitmaps: [Data] = []
        for (filled, frame, mark) in states {
            let image = StatusIconImage.image(filled: filled, frame: frame, mark: mark, colors: colors, description: "x")
            #expect(image.size == NSSize(width: 21, height: 16) && image.isTemplate == (mark == .none))
            let tiff = try #require(image.tiffRepresentation)
            let rep = try #require(NSBitmapImageRep(data: tiff))
            bitmaps.append(try #require(rep.representation(using: .png, properties: [:])))
        }
        #expect(Set(bitmaps).count == states.count, "every state looks different")
    }
}

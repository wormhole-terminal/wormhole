import SwiftUI
import AppKit

// The C interface the Wormhole app (Rust) drives the window through. Everything crosses as JSON text: the app
// sends a snapshot of its settings (`wh_settings_update`) and gets back one event per change of the person's
// (`handler`): {"t":"set","k":key,"v":value}, {"t":"reset","k":key}, {"t":"reset_all"},
// {"t":"action","k":key,"v":button's id}, {"t":"record",…} and {"t":"closed"}. A menu's value is its option's id, a
// button's is its id: neither is ever the words shown. Every function runs on the main thread, and the handler's
// text lives only for the call.

/// The one window.
@MainActor final class SettingsHost: NSObject, NSWindowDelegate {
    static let shared = SettingsHost()
    private(set) var window: NSWindow?
    private var store = HostStore()
    private var model: SettingsModel?
    private(set) var look: WindowLook?

    func open(page: String, behind: Bool, handler: @convention(c) (UnsafePointer<CChar>) -> Void) {
        store.send = { text in text.withCString { handler($0) } }
        if let window {
            show(window, page: page, behind: behind)
            return
        }
        let model = SettingsModel(store: store, page: page.isEmpty ? "Terminal" : page)
        self.model = model
        let host = NSHostingView(rootView: SettingsView(model: model))
        let w = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 840, height: 720),
                         styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView], backing: .buffered, defer: false)
        w.title = "Settings"
        let look = WindowLook(window: w, content: host)
        self.look = look
        w.contentView = look.root
        w.titlebarAppearsTransparent = true
        w.titleVisibility = .hidden
        w.contentMinSize = NSSize(width: 720, height: 520)
        w.isReleasedWhenClosed = false
        w.delegate = self
        w.setFrameAutosaveName("WormholeSettingsWindow")
        if !w.setFrameUsingName("WormholeSettingsWindow") { w.center() }
        window = w
        show(w, page: nil, behind: behind)
    }

    private func show(_ w: NSWindow, page: String?, behind: Bool) {
        if let page, !page.isEmpty { model?.go(page) }
        if behind {
            w.orderBack(nil)
        } else {
            NSApp.activate(ignoringOtherApps: true)
            w.makeKeyAndOrderFront(nil)
        }
    }

    func update(_ json: [String: Any]) {
        store.apply(json)
        if let l = look, let w = json["window"] as? [String: Any] {
            let dark = model?.colourScheme.map { $0 == .dark } ?? (store.values["System:appearance"] != "light")
            l.apply(.init(translucent: w["translucent"] as? Bool ?? false, blur: w["blur"] as? Bool ?? false,
                          glass: w["glass"] as? Bool ?? false, dark: dark, opacity: w["opacity"] as? Double ?? 1))
        }
        // Where the app wants the window to go (the palette's setting, a script): a page, a row to scroll to, a search.
        if let show = json["show"] as? [String: Any] {
            if let page = show["page"] as? String, !page.isEmpty { model?.go(page) }
            if let search = show["search"] as? String { model?.search = search }
            // A row ("<page>:<group>:<row>") is shown by selecting its group.
            if let row = show["scroll"] as? String, !row.isEmpty { let k = row.split(separator: ":"); if k.count > 1 { model?.go("\(k[0])/\(k[1])") } }
        }
    }

    /// The window's content as a PNG, drawn by the view itself (no screen capture), for tests and bug reports.
    func shot(to path: String) -> Bool {
        guard let v = window?.contentView, let rep = v.bitmapImageRepForCachingDisplay(in: v.bounds) else { return false }
        v.cacheDisplay(in: v.bounds, to: rep)
        guard let data = rep.representation(using: .png, properties: [:]) else { return false }
        return (try? data.write(to: URL(fileURLWithPath: path))) != nil
    }

    func windowWillClose(_ notification: Notification) {
        store.emit(["t": "closed"])
        // A reopened window starts from the app's snapshot alone: nothing it kept locally survives the close.
        store = HostStore()
        window?.delegate = nil
        window = nil
        look = nil
        model = nil
    }
}

private func text(_ p: UnsafePointer<CChar>?) -> String? { p.map { String(cString: $0) } }

@_cdecl("wh_settings_open")
public func wh_settings_open(_ page: UnsafePointer<CChar>?, _ behind: Bool, _ handler: @convention(c) (UnsafePointer<CChar>) -> Void) {
    MainActor.assumeIsolated { SettingsHost.shared.open(page: text(page) ?? "", behind: behind, handler: handler) }
}

@_cdecl("wh_settings_update")
public func wh_settings_update(_ json: UnsafePointer<CChar>?) {
    guard let s = text(json), let data = s.data(using: .utf8),
          let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return }
    MainActor.assumeIsolated { SettingsHost.shared.update(object) }
}

@_cdecl("wh_settings_is_open")
public func wh_settings_is_open() -> Bool { MainActor.assumeIsolated { SettingsHost.shared.window != nil } }

@_cdecl("wh_settings_shot")
public func wh_settings_shot(_ path: UnsafePointer<CChar>?) -> Bool {
    guard let path = text(path) else { return false }
    return MainActor.assumeIsolated { SettingsHost.shared.shot(to: path) }
}

@_cdecl("wh_settings_look")
public func wh_settings_look(_ out: UnsafeMutablePointer<CChar>?, _ cap: Int) -> Bool {
    guard let out, cap > 1 else { return false }
    return MainActor.assumeIsolated {
        guard let l = SettingsHost.shared.look else { return false }
        let bytes = Array(l.readout().utf8.prefix(cap - 1))
        for (i, b) in bytes.enumerated() { out[i] = CChar(bitPattern: b) }
        out[bytes.count] = 0
        return true
    }
}

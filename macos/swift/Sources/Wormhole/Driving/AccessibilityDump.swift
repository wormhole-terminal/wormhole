// What VoiceOver gets, read in the process through the same accessibility methods an assistive app's requests arrive
// at: AppKit's views answer them directly and SwiftUI builds its nodes once the application was told an assistive client
// reads it (`wakeSwiftUI`), so SwiftUI's part of the tree is in it: the hosting views' children. Those nodes
// (`AccessibilityNode`) answer the methods without adopting `NSAccessibilityProtocol`, so every element is read by its
// methods' names (`Node`), never by a cast to the protocol: the cast dropped all of SwiftUI's. One element a line, indented
// by depth: role, label, value and states; the groups that are not elements themselves are walked through and not printed,
// as VoiceOver passes over them.

import AppKit

@MainActor
enum AccessibilityDump {
    /// Deep enough for the window's chrome and lists; a deeper tree is cut with a line saying so.
    private static let maxDepth = 40

    static func tree(of window: NSWindow) -> String {
        wakeSwiftUI(window)
        var lines: [String] = []
        walk(window, depth: 0, window: window, into: &lines)
        return lines.isEmpty ? "empty tree" : lines.joined(separator: "\n")
    }

    /// The element VoiceOver finds at a window point, then `levels - 1` of the elements around it.
    static func at(_ point: NSPoint, in window: NSWindow, levels: Int) -> String {
        wakeSwiftUI(window)
        var element = window.accessibilityHitTest(window.convertPoint(toScreen: point))
        var lines: [String] = []
        for _ in 0..<max(1, levels) {
            guard let node = element.flatMap(Node.init) else { break }
            lines.append(describe(node, in: window))
            element = node.read("accessibilityParent")
        }
        return lines.isEmpty ? "nothing at \(point)" : lines.joined(separator: "\n")
    }

    /// One element, asked by its accessibility methods' names (key-value coding boxes a `Bool` and a frame): an AppKit
    /// view and a SwiftUI node answer alike, and one that has no such method answers nil.
    private struct Node {
        let object: NSObject

        init?(_ element: Any) {
            guard let object = element as? NSObject else { return nil }
            self.object = object
        }

        func read(_ method: String) -> Any? { object.responds(to: Selector(method)) ? object.value(forKey: method) : nil }
        func text(_ method: String) -> String? { read(method) as? String }
        func flag(_ method: String) -> Bool? { read(method) as? Bool }
        var frame: NSRect { (read("accessibilityFrame") as? NSValue)?.rectValue ?? .zero }
        func list(_ method: String) -> [Any] { read(method) as? [Any] ?? [] }
    }

    /// Whether the app was told an assistive client is reading it.
    private static var told = false

    /// SwiftUI's hosting views build their accessibility nodes once an assistive client reads the app; with none attached
    /// (`a11y` in a test instance) they answer with AppKit's subviews only. An assistive client says it is there by setting
    /// `AXEnhancedUserInterface` on the application element, as VoiceOver does: set here once, in process, as such a request
    /// would arrive, then the window gets a turn to lay out so the hosting views build their nodes before the walk. A
    /// hosting view then answers `accessibilityChildren` with its nodes (checked on macOS 27 in a bare hosting view).
    private static func wakeSwiftUI(_ window: NSWindow) {
        guard !told else { return }
        told = true
        NSApp.accessibilitySetValue(NSNumber(value: true), forAttribute: NSAccessibility.Attribute(rawValue: "AXEnhancedUserInterface"))
        window.contentView?.layoutSubtreeIfNeeded()
        RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.2))
    }

    private static func walk(_ element: Any, depth: Int, window: NSWindow, into lines: inout [String]) {
        guard let node = Node(element) else { return }
        var depth = depth
        if node.flag("isAccessibilityElement") == true {
            lines.append(String(repeating: "  ", count: depth) + describe(node, in: window))
            depth += 1
        }
        guard depth < maxDepth else { return lines.append(String(repeating: "  ", count: depth) + "(deeper not shown)") }
        for child in children(of: node) { walk(child, depth: depth, window: window, into: &lines) }
    }

    /// A node's children as the accessibility methods give them. A SwiftUI hosting view vends its nodes through
    /// `accessibilityChildren`, or through the navigation order when that is empty; a view that answers neither is
    /// walked by its subviews so nothing under it is skipped.
    private static func children(of node: Node) -> [Any] {
        let children = node.list("accessibilityChildren")
        if !children.isEmpty { return children }
        let ordered = node.list("accessibilityChildrenInNavigationOrder")
        return ordered.isEmpty ? (node.object as? NSView)?.subviews ?? [] : ordered
    }

    /// `role[/subrole] "label" = value (help) [focused] [off] @x,y wxh` (the frame in the window's points, from its top left).
    private static func describe(_ node: Node, in window: NSWindow) -> String {
        let role = node.text("accessibilityRole")?.replacingOccurrences(of: "AX", with: "") ?? "?"
        let subrole = node.text("accessibilitySubrole")?.replacingOccurrences(of: "AX", with: "")
        var text = role + (subrole.map { "/\($0)" } ?? "")
        // An element with no label of its own may be named by another (a picker's words beside its radio group): VoiceOver
        // speaks that element's words as the name, so they are the label here.
        let own = node.text("accessibilityLabel")
        let named = node.read("accessibilityTitleUIElement").flatMap(Node.init)
        let label = own?.isEmpty == false ? own : named.flatMap { $0.text("accessibilityLabel") ?? $0.read("accessibilityValue") as? String }
        if let label, !label.isEmpty { text += " \"\(label)\"" }
        if let title = node.text("accessibilityTitle"), !title.isEmpty, title != label { text += " title=\"\(title)\"" }
        if let value = node.read("accessibilityValue").map({ "\($0)" }), !value.isEmpty {
            let line = value.replacingOccurrences(of: "\n", with: "⏎")
            text += " = \(line.count > 60 ? line.prefix(60) + "…" : line)"
        }
        if let help = node.text("accessibilityHelp"), !help.isEmpty { text += " (\(help))" }
        if node.flag("isAccessibilityFocused") == true { text += " [focused]" }
        if node.flag("isAccessibilityEnabled") == false { text += " [off]" }
        let screen = node.frame
        if screen != .zero {
            let frame = window.convertFromScreen(screen), height = window.contentView?.bounds.height ?? frame.maxY
            text += " @\(Int(frame.minX)),\(Int(height - frame.maxY)) \(Int(frame.width))x\(Int(frame.height))"
        }
        return text
    }
}

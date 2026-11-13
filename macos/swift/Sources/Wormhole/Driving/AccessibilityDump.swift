// What VoiceOver gets, read in the process through the same NSAccessibility protocol an assistive app's requests
// arrive at: AppKit's views answer it directly and SwiftUI builds its nodes when they are first asked for, as they
// are for VoiceOver (so SwiftUI's part of the tree is in it: the hosting views' children). One element a line, indented by depth: role, label, value and states; the groups that are not
// elements themselves are walked through and not printed, as VoiceOver passes over them.

import AppKit

@MainActor
enum AccessibilityDump {
    /// Deep enough for the window's chrome and lists; a deeper tree is cut with a line saying so.
    private static let maxDepth = 40

    static func tree(of window: NSWindow) -> String {
        var lines: [String] = []
        walk(window, depth: 0, window: window, into: &lines)
        return lines.isEmpty ? "empty tree" : lines.joined(separator: "\n")
    }

    /// The element VoiceOver finds at a window point, then `levels - 1` of the elements around it.
    static func at(_ point: NSPoint, in window: NSWindow, levels: Int) -> String {
        var element = window.accessibilityHitTest(window.convertPoint(toScreen: point))
        var lines: [String] = []
        for _ in 0..<max(1, levels) {
            guard let node = element as? NSAccessibilityProtocol else { break }
            lines.append(describe(node, in: window))
            element = node.accessibilityParent()
        }
        return lines.isEmpty ? "nothing at \(point)" : lines.joined(separator: "\n")
    }

    private static func walk(_ element: Any, depth: Int, window: NSWindow, into lines: inout [String]) {
        guard let node = element as? NSAccessibilityProtocol else { return }
        var depth = depth
        if node.isAccessibilityElement() == true {
            lines.append(String(repeating: "  ", count: depth) + describe(node, in: window))
            depth += 1
        }
        guard depth < maxDepth else { return lines.append(String(repeating: "  ", count: depth) + "(deeper not shown)") }
        for child in children(of: node) { walk(child, depth: depth, window: window, into: &lines) }
    }

    /// A node's children as AppKit's accessibility API gives them. A SwiftUI hosting view vends its nodes through
    /// `accessibilityChildren`, or through the navigation order when that is empty; a view that answers neither is
    /// walked by its subviews so nothing under it is skipped.
    private static func children(of node: NSAccessibilityProtocol) -> [Any] {
        if let children = node.accessibilityChildren(), !children.isEmpty { return children }
        if let ordered = node.accessibilityChildrenInNavigationOrder(), !ordered.isEmpty { return ordered }
        return (node as? NSView)?.subviews ?? []
    }

    /// `role[/subrole] "label" = value (help) [focused] [off] @x,y wxh` (the frame in the window's points, from its top left).
    private static func describe(_ node: NSAccessibilityProtocol, in window: NSWindow) -> String {
        let role = node.accessibilityRole()?.rawValue.replacingOccurrences(of: "AX", with: "") ?? "?"
        let subrole = node.accessibilitySubrole()?.rawValue.replacingOccurrences(of: "AX", with: "")
        var text = role + (subrole.map { "/\($0)" } ?? "")
        if let label = node.accessibilityLabel(), !label.isEmpty { text += " \"\(label)\"" }
        if let title = node.accessibilityTitle(), !title.isEmpty, title != node.accessibilityLabel() { text += " title=\"\(title)\"" }
        if let value = node.accessibilityValue().map({ "\($0)" }), !value.isEmpty {
            let line = value.replacingOccurrences(of: "\n", with: "⏎")
            text += " = \(line.count > 60 ? line.prefix(60) + "…" : line)"
        }
        if let help = node.accessibilityHelp(), !help.isEmpty { text += " (\(help))" }
        if node.isAccessibilityFocused() == true { text += " [focused]" }
        if node.isAccessibilityEnabled() == false { text += " [off]" }
        let screen = node.accessibilityFrame()
        if screen != .zero {
            let frame = window.convertFromScreen(screen), height = window.contentView?.bounds.height ?? frame.maxY
            text += " @\(Int(frame.minX)),\(Int(height - frame.maxY)) \(Int(frame.width))x\(Int(frame.height))"
        }
        return text
    }
}

// A `MenuNode` tree as an `NSMenu`: native items, native submenus and section headers, native Services. The
// trailing time of a row is an attributed second part of the title (a tab stop would not survive a Dynamic Type
// change); a subtitle is the item's own (macOS 14.4+). Every choice goes through one `MenuDispatcher`.

import AppKit
import PlatformModel

@MainActor
public struct NSMenuFactory {
    public let dispatcher: MenuDispatcher
    public var style: MenuStyle
    /// Items whose action AppKit answers itself, by the command (Quit, Copy, Zoom ...), with their target.
    public var standard: [MenuCommand: Selector]

    public init(dispatcher: MenuDispatcher, style: MenuStyle = .themed(), standard: [MenuCommand: Selector] = [:]) {
        self.dispatcher = dispatcher
        self.style = style
        self.standard = standard
    }

    public func menu(title: String, _ nodes: [MenuNode]) -> NSMenu {
        let menu = NSMenu(title: title)
        fill(menu, with: nodes)
        return menu
    }

    /// Replaces what `menu` holds, for a menu that stays (a status item's, rebuilt as it opens).
    public func fill(_ menu: NSMenu, with nodes: [MenuNode]) {
        menu.removeAllItems()
        menu.autoenablesItems = false
        for node in nodes { append(node, to: menu) }
    }

    private func append(_ node: MenuNode, to menu: NSMenu) {
        switch node {
        case .separator: menu.addItem(.separator())
        case .header(let title): menu.addItem(NSMenuItem.sectionHeader(title: title))
        case .services:
            let holder = NSMenuItem(title: "Services", action: nil, keyEquivalent: "")
            let services = NSMenu(title: "Services")
            holder.submenu = services
            NSApp?.servicesMenu = services
            menu.addItem(holder)
        case .placeholder(let title):
            let item = NSMenuItem(title: title, action: nil, keyEquivalent: "")
            item.isEnabled = false
            menu.addItem(item)
        case .submenu(let title, let items, let enabled, let trailing):
            let holder = NSMenuItem(title: title, action: nil, keyEquivalent: "")
            if let trailing { holder.attributedTitle = Self.titled(title, trailing: trailing) }
            holder.submenu = self.menu(title: title, items)
            holder.isEnabled = enabled
            menu.addItem(holder)
        case .swatches(let row):
            let holder = NSMenuItem()
            holder.view = SwatchMenuView(row: row, style: style) { [dispatcher] action in dispatcher.pick(.context(action)) }
            menu.addItem(holder)
        case .item(let item): menu.addItem(self.item(item))
        }
    }

    func item(_ model: MenuItem) -> NSMenuItem {
        let item = NSMenuItem(title: model.title, action: #selector(MenuDispatcher.choose(_:)), keyEquivalent: "")
        item.target = dispatcher
        item.representedObject = CommandBox(model.command)
        if let selector = standard[model.command] {
            // AppKit's own: the responder chain answers, so Copy goes to the field that has the focus.
            item.action = selector
            item.target = nil
        }
        if let key = model.key.flatMap(KeyEquivalent.init) { key.apply(to: item) }
        item.state = model.checked ? .on : .off
        item.isEnabled = model.enabled
        item.image = model.icon.flatMap { MenuIcons.image(for: $0, style: style) }
        item.toolTip = model.subtitle
        if let subtitle = model.subtitle { item.subtitle = subtitle }
        if let trailing = model.trailing { item.attributedTitle = Self.titled(model.title, trailing: trailing) }
        return item
    }

    /// The title with its time after it, in secondary colour and monospaced digits so the times line up.
    static func titled(_ title: String, trailing: String) -> NSAttributedString {
        let text = NSMutableAttributedString(string: title, attributes: [.font: NSFont.menuFont(ofSize: 0)])
        text.append(NSAttributedString(string: "   " + trailing, attributes: [
            .font: NSFont.monospacedDigitSystemFont(ofSize: NSFont.smallSystemFontSize, weight: .regular),
            .foregroundColor: NSColor.secondaryLabelColor,
        ]))
        return text
    }
}

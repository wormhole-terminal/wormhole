// text-file: the control folder's `title-menu` (dev tooling), not interface text
// The title row's right-click menu (`TitleRowMenu`) as items, for the control folder: each with its title, its check, whether
// it is on, and what choosing it runs (the same `TitleActions` the menu's toggles and buttons call). `TitleRowMenu`'s body
// draws these in this order: a header, a toggle per part that can be hidden, a divider, Reset and Customize.

import PanesModel

@MainActor
public enum TitleMenuReadout {
    public struct Item {
        public let title: String
        /// Checked (a shown part), unchecked, or nil for an item with no check.
        public let checked: Bool?
        public let enabled: Bool
        public let run: () -> Void
    }

    /// The menu's items in its order; nil marks the divider.
    public static func items(row: TitleRow, actions: TitleActions) -> [Item?] {
        var items: [Item?] = [Item(title: PanesText.showInTitleRow, checked: nil, enabled: false, run: {})]
        for part in TitlePart.allCases.filter(\.hideable) {
            let shown = row.shows(part)
            items.append(Item(title: PanesText.partName(part), checked: shown, enabled: actions.canEditRow, run: { actions.setPart(part, !shown) }))
        }
        items.append(nil)
        items.append(Item(title: PanesText.resetTitleRow, checked: nil, enabled: !row.isDefault && actions.canEditRow, run: actions.resetRow))
        items.append(Item(title: PanesText.customizeTitleRow, checked: nil, enabled: actions.canEditRow, run: actions.customizeRow))
        return items
    }

    /// The items as `menu` prints a native one: `n. title [x] [off]`, `---` for the divider.
    public static func text(_ items: [Item?]) -> String {
        var number = 0
        return items.map { item in
            guard let item else { return "---" }
            number += 1
            return "\(number). \(item.title)" + (item.checked == true ? " [x]" : "") + (item.enabled ? "" : " [off]")
        }.joined(separator: "\n")
    }
}

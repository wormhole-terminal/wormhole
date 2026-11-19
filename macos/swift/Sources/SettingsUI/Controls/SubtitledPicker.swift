// A select whose menu says under each choice what it does (the Default shell's "Login shell: /bin/zsh"): the system's pop-up
// button, because a SwiftUI picker's menu has no second line; macOS gives an item its own subtitle (14.4+). The button
// shows the name chosen, as the GPUI select does, and the menu both lines.

import AppKit
import SettingsModel
import SwiftUI

struct SubtitledPicker: NSViewRepresentable {
    let options: [SettingOption]
    let selected: JSONValue
    let label: String
    let width: CGFloat
    let onSelect: (JSONValue) -> Void

    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeNSView(context: Context) -> NSPopUpButton {
        let button = NSPopUpButton(frame: .zero, pullsDown: false)
        button.target = context.coordinator
        button.action = #selector(Coordinator.picked(_:))
        return button
    }

    func updateNSView(_ button: NSPopUpButton, context: Context) {
        context.coordinator.options = options
        context.coordinator.onSelect = onSelect
        button.removeAllItems()
        for option in options {
            let item = NSMenuItem(title: option.label, action: nil, keyEquivalent: "")
            if let detail = option.detail { item.subtitle = detail }
            button.menu?.addItem(item)
        }
        if let index = options.firstIndex(where: { $0.value == selected }) { button.selectItem(at: index) }
        button.setAccessibilityLabel(label)
    }

    func sizeThatFits(_ proposal: ProposedViewSize, nsView: NSPopUpButton, context: Context) -> CGSize? {
        CGSize(width: width, height: nsView.fittingSize.height)
    }

    @MainActor
    final class Coordinator: NSObject {
        var options: [SettingOption] = []
        var onSelect: (JSONValue) -> Void = { _ in }

        @objc func picked(_ button: NSPopUpButton) {
            guard options.indices.contains(button.indexOfSelectedItem) else { return }
            onSelect(options[button.indexOfSelectedItem].value)
        }
    }
}

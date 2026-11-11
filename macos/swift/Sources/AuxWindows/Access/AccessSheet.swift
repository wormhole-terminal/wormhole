// The control API's approval question (`agent_access/sheet.rs`): a native sheet on the front window, with the
// program's facts in a monospace block as the alert's accessory. The buttons are what `AccessDisplay` lists, first
// the one that backs out; the answer is the button pressed, or `nil` when the sheet was taken away (the window it
// was about closed, or the request was withdrawn).

import AppKit
import AuxModel

@MainActor
public final class AccessSheet {
    private let alert = NSAlert()
    /// The alert's buttons as the person reads them, first the default (for tests: the alert is the system's window).
    var buttonTitles: [String] { alert.buttons.map(\.title) }
    private let display: AccessDisplay
    private weak var window: NSWindow?

    public init(_ display: AccessDisplay) {
        self.display = display
        alert.messageText = display.title
        alert.informativeText = display.detail
        alert.alertStyle = .informational
        // The first button is the default (Return): the one that backs out, so Return never grants access.
        for button in display.buttons { alert.addButton(withTitle: button.label) }
        alert.buttons.first?.keyEquivalent = "\r"
        if !display.accessory.isEmpty { alert.accessoryView = Self.facts(display.accessory) }
    }

    public convenience init(_ request: AccessRequest) { self.init(AccessDisplay.of(request)) }

    /// Shows the sheet on `window` (a modal alert without one); `nil` when it was withdrawn.
    public func ask(in window: NSWindow?) async -> AccessButton? {
        self.window = window
        if let pending = PendingQuestions.current {
            // The first button backs out; the next is the first way in.
            let place = await pending.hold(
                title: display.title, detail: display.detail, buttons: display.buttons.map(\.label), confirm: 1, cancel: 0)
            return display.buttons.indices.contains(place) ? display.buttons[place] : nil
        }
        // Esc is Don't Allow: the system maps Esc only to a button titled Cancel, and Return is Don't Allow too.
        let escape = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [alert] event in
            guard event.keyCode == Self.escapeKeyCode, event.window === alert.window else { return event }
            alert.buttons.first?.performClick(nil)
            return nil
        }
        defer { escape.map(NSEvent.removeMonitor) }
        let response = window != nil ? await alert.beginSheetModal(for: window!) : alert.runModal()
        guard response != .cancel else { return nil }
        let place = response.rawValue - NSApplication.ModalResponse.alertFirstButtonReturn.rawValue
        return display.buttons.indices.contains(place) ? display.buttons[place] : nil
    }

    /// Takes the sheet away without an answer.
    public func withdraw() {
        PendingQuestions.current?.withdraw(title: display.title)
        guard let window, window.attachedSheet === alert.window else { return }
        window.endSheet(alert.window, returnCode: .cancel)
    }

    /// The facts in the code font, selectable, as wide as the alert's text column.
    private static func facts(_ text: String) -> NSView {
        let field = NSTextField(wrappingLabelWithString: text)
        field.font = .monospacedSystemFont(ofSize: NSFont.smallSystemFontSize, weight: .regular)
        field.isSelectable = true
        field.setAccessibilityLabel(text)
        field.preferredMaxLayoutWidth = factsWidth
        field.frame.size = field.fittingSize
        return field
    }

    private static let escapeKeyCode: UInt16 = 53

    /// The alert's text column is about this wide (the system's own, 3.5 in at the default).
    private static let factsWidth: CGFloat = 300
}

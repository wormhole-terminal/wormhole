// Print… (⌘P): the terminal's text on paper with `NSPrintOperation`, so the print panel, PDF menu and page setup are
// the system's. The view is a text view in the terminal's own monospaced font that wraps at the page's width; the
// job is named after the terminal. Like Terminal.app, lines are not cut: a long line wraps.

import AppKit

@MainActor
public enum PrintableText {
    /// A text view laid out for `info`'s page, with `text` in `font`.
    public static func view(text: String, font: NSFont, info: NSPrintInfo) -> NSTextView {
        let width = info.paperSize.width - info.leftMargin - info.rightMargin
        let view = NSTextView(frame: NSRect(x: 0, y: 0, width: width, height: 1))
        view.isEditable = false
        view.textContainerInset = .zero
        view.textContainer?.lineFragmentPadding = 0
        view.isVerticallyResizable = true
        view.font = font
        view.textColor = .black
        view.string = text
        view.sizeToFit()
        return view
    }

    /// The print operation for `text`; the caller runs it (`runModal(for:…)` as a sheet, or `run()`).
    public static func operation(text: String, title: String, font: NSFont, info: NSPrintInfo = .shared) -> NSPrintOperation {
        let operation = NSPrintOperation(view: view(text: text, font: font, info: info), printInfo: info)
        operation.jobTitle = title
        operation.showsPrintPanel = true
        operation.showsProgressPanel = true
        return operation
    }

    /// Runs it as a sheet over `window`, or modal.
    public static func print(text: String, title: String, font: NSFont, over window: NSWindow?) {
        let operation = operation(text: text, title: title, font: font)
        if let window {
            operation.runModal(for: window, delegate: nil, didRun: nil, contextInfo: nil)
        } else {
            operation.run()
        }
    }

    /// The same job without the panel, saved as a PDF at `url`: how a script checks what would come out on paper.
    @discardableResult
    public static func savePDF(text: String, title: String, font: NSFont, to url: URL) -> Bool {
        let info = NSPrintInfo()
        info.jobDisposition = .save
        info.dictionary()[NSPrintInfo.AttributeKey.jobSavingURL] = url
        let operation = operation(text: text, title: title, font: font, info: info)
        operation.showsPrintPanel = false
        operation.showsProgressPanel = false
        return operation.run()
    }
}

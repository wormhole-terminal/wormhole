// Shell ▸ Print… and Export Text As…: the terminal's text on paper or in a file (`TerminalUI/Output`).

import AppKit
import PlatformModel
import TerminalCore
import TerminalMetal
import TerminalUI
import WindowUI

extension ActionRouter {
    func outputActions() -> [ActionID: Handler] {
        [
            "print": { [unowned self] in app.printTerminal() },
            "exportText": { [unowned self] in app.exportTerminalText() },
        ]
    }
}

/// The selected terminal's text, read from its source when asked.
@MainActor
final class ViewContents: TerminalContents {
    let view: MetalTerminalView
    let title: String

    init(view: MetalTerminalView, title: String) {
        self.view = view
        self.title = title
    }

    func text(_ scope: ContentsScope) -> String? {
        switch scope {
        case .screen: view.source.plainText(history: false)
        case .everything: view.source.plainText(history: true)
        case .selection: view.source.interaction?.selectionText()
        }
    }
}

extension AppController {
    /// What is selected, else the screen: a person who selected something and printed means that. `pdf`: no panel, the
    /// job saved there (a script's check of the output).
    @discardableResult
    func printTerminal(pdf: URL? = nil) -> Bool {
        guard let (contents, view) = selectedContents(), let text = contents.text(.selection) ?? contents.text(.screen)
        else { return false }
        let font = view.renderer.configuration.fonts.printFont(size: Self.printPointSize)
        if let pdf { return PrintableText.savePDF(text: text, title: contents.title, font: font, to: pdf) }
        PrintableText.print(text: text, title: contents.title, font: font, over: window)
        return true
    }

    /// What is selected, else everything the terminal keeps.
    func exportTerminalText() {
        guard let (contents, _) = selectedContents(), let text = contents.text(.selection) ?? contents.text(.everything)
        else { return }
        // What was written is said to a script driving the instance (`reply` is silent without one).
        let scratch = policy.touchesSystem ? nil : dataDirectory.appendingPathComponent("exports")
        if let scratch { try? FileManager.default.createDirectory(at: scratch, withIntermediateDirectories: true) }
        TextExporter.export(text, title: contents.title, over: window, directory: scratch) { [weak self] in self?.reply("export \($0?.path ?? "cancelled")") } // text: a reply line for a test driver
    }

    /// The size text is set at on paper, in points (Terminal.app prints at its window's size; paper wants less).
    static let printPointSize: CGFloat = 10

    private func selectedContents() -> (ViewContents, MetalTerminalView)? {
        guard let view = selectedView, let id = hub.state.selected else { return nil }
        let title = (hub.state.terminals + hub.state.all).first { $0.id == id }?.title ?? ""
        return (ViewContents(view: view, title: title), view)
    }
}

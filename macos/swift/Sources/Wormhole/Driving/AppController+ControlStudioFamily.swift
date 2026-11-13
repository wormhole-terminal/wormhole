// text-file: a test driver's replies on the control folder, not interface text
// GPUI devctl's `studio-*` commands under their own names, over the open Studio's session and document (the model a click
// reaches): `studio-select <token>`, `studio-set <value>`, `studio-type <text>`, `studio-save [name]`, `studio-filter
// <all|yours|base|low> [text]`, `studio-area <area>`, `studio-rename <name>`, `studio-undo`, `studio-redo`, `studio-revert`,
// `studio-appearance light|dark`, `studio-add-appearance [light|dark]`, `studio-chip [token]`, `studio-update-from
// [apply|cancel]`, `studio-versions [select <n>|restore [<n>]|close]`, `studio-use`, `studio-menu [save|duplicate|json|project]`,
// `studio-theme-menu`, `studio-open own:<stem>|builtin:<name>|from:<name>|new|project`, and the window's `studio-resize <w> <h>`,
// `studio-scroll <y>`, `studio-shot <name>` and `studio-frames <n> [scroll]`. `studio-do` and `studio-out` stay as they are.
//
// What may ask a question (saving over a file, opening another theme over unsaved edits) runs beside the batch and replies
// when it is done: the question is held for `question confirm|cancel`, which must be able to run meanwhile.

import AppKit
import DesignSystem
import StudioModel
import StudioUI

extension AppController {
    func runStudioFamily(command: String, argument: String) async -> Bool {
        guard command.hasPrefix("studio-"), command != "studio-do", command != "studio-out" else { return false }
        guard let session = studio.session else { reply("error: the Studio is not open (`studio`)"); return true }
        let document = session.document
        let words = argument.split(separator: " ").map(String.init)
        var said = ""
        switch command {
        case "studio-select": document.selected = argument.isEmpty ? nil : argument
        case "studio-set", "studio-type":
            // Into the selected token's field: `studio-set` presses Return (the draft takes it or the field says why not),
            // `studio-type` only types (what the field says of it as it stands).
            guard let token = document.selected else { reply("error: no token is selected (studio-select <token>)"); return true }
            if command == "studio-set" { said = " ok=\(document.set(token, to: argument))" } else { document.validate(token, typing: argument) }
            said += " field=\(document.fieldError?.message ?? "-")"
        case "studio-filter":
            let shows: [String: TokenFilter.Show] = ["all": .all, "yours": .yours, "base": .base, "low": .lowContrast]
            guard let show = words.first.flatMap({ shows[$0] }) else { reply("error: studio-filter all|yours|base|low [text]"); return true }
            document.filter.show = show
            document.filter.query = words.dropFirst().joined(separator: " ")
            said = " shown=\(document.visibleGroups.reduce(0) { $0 + $1.tokens.count })"
        case "studio-area": document.filter.area = StudioArea(rawValue: argument) ?? .all
        case "studio-rename": document.rename(argument)
        case "studio-undo": document.undo()
        case "studio-redo": document.redo()
        case "studio-revert": document.revertToSaved()
        case "studio-appearance": document.switchVersion(to: argument)
        case "studio-add-appearance":
            // The one the theme lacks, unless one is named.
            guard let wanted = ["light": ThemeAppearance.light, "dark": .dark][argument] ?? document.missingAppearance else {
                reply("error: the theme has both appearances")
                return true
            }
            session.addAppearance(wanted)
        case "studio-chip":
            // The chip of a pair: the value moves between both appearances and this one alone.
            guard let token = words.first ?? document.selected else { reply("error: studio-chip [token]"); return true }
            document.move(token, to: document.place(of: token) == .top ? .version : .top)
            said = " place=\(document.place(of: token))"
        case "studio-update-from":
            if argument == "cancel" { session.showsUpdate = false } else { session.askUpdate() }
            if argument == "apply" { session.applyUpdate() }
            said = " sheet=\(session.showsUpdate) update=\(document.update.state) changes=\(document.update.changes.count)"
        case "studio-versions": said = studioVersions(words, document)
        case "studio-theme-menu": _ = runStudioSession(["menu"]); return true
        case "studio-save", "studio-use", "studio-menu", "studio-open": return studioAsking(command, words, session)
        default: return await studioWindow(command, words, session)
        }
        reply(studioState(command, argument, said, session))
        return true
    }

    /// The Versions list: the rows (row 0 is the file as it is now), a row previewed in every window as selecting it does,
    /// a row restored as unsaved edits, or the preview back to the file as it is (closing the panel).
    private func studioVersions(_ words: [String], _ document: StudioDocument) -> String {
        let rows = document.versionRows()
        let index = words.count > 1 ? Int(words[1]) : nil
        switch words.first {
        case "select":
            guard let index, rows.indices.contains(index) else { return " error=no such row" }
            document.preview(rows[index])
        case "restore":
            // Without a number: the newest save behind the file.
            guard rows.indices.contains(index ?? 1) else { return " error=no such row" }
            document.restore(rows[index ?? 1])
        case "close": rows.first.map(document.preview)
        default: break
        }
        return " versions=\(rows.count)\n" + rows.enumerated().map { "  \($0.offset): \($0.element.id?.lastPathComponent ?? "now") \($0.element.summary)\($0.element.pinned ? " pinned" : "")" }.joined(separator: "\n")
    }

    /// The commands that may ask: they run beside the batch and reply when done.
    private func studioAsking(_ command: String, _ words: [String], _ session: StudioSession) -> Bool {
        let argument = words.joined(separator: " ")
        if command == "studio-menu", words.isEmpty { return runStudioSession(["menu"]) }
        // Copy JSON goes to the scratch folder's file, never the person's clipboard (`studio-out`).
        if command == "studio-menu", argument == "json" { runStudioOutputs("copy-json"); return true }
        Task { @MainActor in
            switch (command, words.first) {
            case ("studio-save", nil), ("studio-menu", "save"): await session.save()
            case ("studio-save", _):
                // A name: a copy without a file takes it and is saved; a theme with a file is renamed, then saved.
                await session.save()
                if session.renaming { await session.finishRenaming(argument) } else {
                    session.document.rename(argument)
                    await session.save()
                }
            case ("studio-use", _): await session.use()
            case ("studio-menu", "duplicate"): session.duplicate()
            case ("studio-menu", "project"): session.useInProject()
            case ("studio-open", "new"): await session.openNew()
            case ("studio-open", "project"): await session.openProject()
            case ("studio-open", let target?): await studioOpen(target, session)
            default: break
            }
            reply(studioState(command, argument, session.renaming ? " renaming=true (name it: studio-save <name>)" : "", session))
        }
        return true
    }

    /// `own:<file stem>` and `builtin:<name>` open that theme; `from:<name>` starts a new one from it (one of the person's
    /// by its file stem or name, else a built-in).
    private func studioOpen(_ target: String, _ session: StudioSession) async {
        let parts = target.split(separator: ":", maxSplits: 1).map(String.init)
        guard parts.count == 2 else { return }
        let own = session.themeList.own.first {
            URL(fileURLWithPath: $0.value).deletingPathExtension().lastPathComponent == parts[1] || $0.name == parts[1]
        }
        let builtin = ThemeChoice(name: parts[1], value: parts[1])
        switch (parts[0], own) {
        case ("own", let choice?): await session.open(choice, builtin: false)
        case ("builtin", _): await session.open(builtin, builtin: true)
        case ("from", let choice?): await session.newFrom(choice, builtin: false)
        case ("from", nil): await session.newFrom(builtin, builtin: true)
        default: session.document.fail("no theme \(target)")
        }
    }

    /// The window's own: its size, the gallery's wheel, a picture of it, and how long it takes to draw.
    private func studioWindow(_ command: String, _ words: [String], _ session: StudioSession) async -> Bool {
        guard let window = session.window, let content = window.contentView else { reply("error: the Studio has no window"); return true }
        let numbers = words.compactMap(Double.init)
        // The gallery is the middle column: the wheel goes to the window's centre.
        let centre = content.convert(NSPoint(x: content.bounds.midX, y: content.bounds.midY), to: nil)
        switch command {
        case "studio-resize":
            guard numbers.count == 2 else { reply("error: studio-resize W H"); return true }
            window.setContentSize(NSSize(width: numbers[0], height: numbers[1]))
            reply("studio-resize \(Int(content.bounds.width))x\(Int(content.bounds.height))")
        case "studio-scroll":
            let total = numbers.first ?? 0, count = max(1, Int((abs(total) / 20).rounded(.up)))
            var last = ""
            for _ in 0..<count {
                last = injector.scroll(centre, in: window, dy: total / Double(count))
                await pause(16)
            }
            reply("studio-scroll \(Int(total)): \(last)")
        case "studio-shot":
            guard let folder = options.devctl else { reply("error: no control folder"); return true }
            let name = Self.fileName(words.first, default: "studio")
            reply(windowShot(to: folder.appendingPathComponent("\(name).png").path, titled: window.title))
        case "studio-frames":
            // The window drawn n times, each timed, to `frames.txt`; with `scroll` a wheel tick over the gallery before each.
            let frames = max(1, Int(numbers.first ?? 40)), scrolls = words.contains("scroll")
            var took: [Double] = []
            for _ in 0..<frames {
                if scrolls { _ = injector.scroll(centre, in: window, dy: 20) }
                let start = CFAbsoluteTimeGetCurrent()
                content.needsDisplay = true
                content.displayIfNeeded()
                took.append((CFAbsoluteTimeGetCurrent() - start) * 1000)
                await pause(2)
            }
            let sorted = took.sorted()
            let line = String(
                format: "studio-frames %ld%@: avg=%.3fms p50=%.3fms max=%.3fms", frames, scrolls ? " scroll" : "", took.reduce(0, +) / Double(frames),
                sorted[frames / 2], sorted[frames - 1])
            devctl?.append(line, to: "frames.txt")
            reply(line)
        default:
            reply("error: no command \(command)")
        }
        return true
    }

    /// A file name of the control folder: letters, digits, `.`, `_` and `-` only (GPUI's `file_name`), else the default.
    static func fileName(_ wanted: String?, default fallback: String) -> String {
        guard let wanted, !wanted.isEmpty, wanted.allSatisfy({ $0.isASCII && ($0.isLetter || $0.isNumber || "._-".contains($0)) }), !wanted.hasPrefix(".") else {
            return fallback
        }
        return wanted
    }

    private func studioState(_ command: String, _ argument: String, _ said: String, _ session: StudioSession) -> String {
        let document = session.document
        return "\(command)\(argument.isEmpty ? "" : " \(argument)")\(said): name=\(document.name) selected=\(document.selected ?? "-") shown=\(document.shown ?? "-") dirty=\(document.isDirty) "
            + "changes=\(document.changeCount) counts=\(document.counts) area=\(document.filter.area.rawValue) inuse=\(document.inUse) "
            + "undo=\(document.undoLabel ?? "-") redo=\(document.redoLabel ?? "-") notice=\(document.notice.map { "\($0)" } ?? "-")"
    }
}

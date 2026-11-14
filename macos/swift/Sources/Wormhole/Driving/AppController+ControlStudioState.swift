// text-file: a test driver's replies on the control folder, not interface text
// The Studio's line at the end of `state` while it is open, in the words of GPUI's (`studio::devctl::state`): whether it
// has edits, whether a question (a system sheet no shot shows) or the name of a copy is waited for, what is edited, the
// undo and redo depths, the file's format, the appearance shown, Update from's state, the sheet up, whether the theme is
// the one in use, and the selected token with where its value comes from and which appearances it is for.

import AuxWindows
import StudioModel
import StudioUI

extension AppController {
    /// `\nstudio …`, or nothing while the Studio is closed.
    var studioStateLine: String {
        guard let session = studio.session else { return "" }
        let document = session.document
        let format = document.isNewer ? "newer:\(document.report?.skipped ?? 0)" : document.file.isV1 ? "v1" : "v0"
        let update = switch document.update.state {
        case .none: "none"
        case .missing: "missing"
        case .current: "current"
        case .changed: "changed:\(document.update.changes.count)"
        }
        // The sheet up over the Studio: Update from… or Versions (`modal_word`).
        return "\nstudio dirty=\(document.isDirty) asking=\(PendingQuestions.current != nil || session.renaming) subject=\(Self.subjectWord(document.subject)) "
            + "undo=\(document.undoDepth) redo=\(document.redoDepth) format=\(format) shown=\(document.shown ?? "-") update=\(update) "
            + "modal=\(session.showsUpdate ? "update" : session.showsVersions ? "versions" : "-") inuse=\(document.inUse) \(Self.tokenWord(document))"
    }

    /// `own:<file stem>`, `untitled:<from>` (spaces as `_`) or `project`, as GPUI's `Subject::word`.
    private static func subjectWord(_ subject: ThemeSubject) -> String {
        switch subject {
        case .own(let url): "own:\(url.deletingPathExtension().lastPathComponent)"
        case .untitled(let from): "untitled:\(from.replacingOccurrences(of: " ", with: "_"))"
        case .project: "project"
        }
    }

    /// `token=<name> origin=yours|base|worked_out reach=both|this|-` for the selected token (`token_word`).
    private static func tokenWord(_ document: StudioDocument) -> String {
        guard let token = document.selected else { return "token=-" }
        let origin = switch document.origin(of: token) {
        case .yours: "yours"
        case .base: "base"
        case .workedOut: "worked_out"
        }
        let reach = document.file.versions.count < 2 ? "-" : document.place(of: token) == .top ? "both" : "this"
        return "token=\(token) origin=\(origin) reach=\(reach)"
    }
}

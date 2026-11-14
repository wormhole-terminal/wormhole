// The control folder's later command families, in one place so the entry (`run(line:)`) names a single hook: each family
// is a file of its own (`AppController+Control<Family>.swift`) with a `run<Family>` that says whether the command was its.

import Foundation

extension AppController {
    /// False when `command` is not one of the families listed here.
    func runExtra(command: String, argument: String) async -> Bool {
        if await runNotes(command: command, argument: argument) { return true }
        if await runSidebar(command: command, argument: argument) { return true }
        if runProfiles(command: command, argument: argument) { return true }
        if runNotices(command: command, argument: argument) { return true }
        if await runPages(command: command, argument: argument) { return true }
        if runMenus(command: command, argument: argument) { return true }
        if runDrag(command: command, argument: argument) { return true }
        if runReadouts(command: command, argument: argument) { return true }
        return false
    }
}

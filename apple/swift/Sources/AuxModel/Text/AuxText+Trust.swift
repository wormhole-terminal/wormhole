// The project trust window's words (`features/project/trust_window.rs`, `panel.rs`).

import Foundation

extension AuxText {
    public static var trustWindowTitle: String { tr("trust-window-title", "Project commands") }
    public static var trustTrusted: String { tr("trust-badge-trusted", "trusted") }
    public static var trustChanged: String { tr("trust-badge-changed", "changed") }
    public static var trustNot: String { tr("trust-badge-not", "not trusted") }
    public static var trustSummaryTrusted: String {
        tr("trust-summary-trusted", "Trusted: actions, the terminal layout and project env may run.")
    }
    public static var trustSummaryChanged: String {
        tr("trust-summary-changed", "Files changed since you trusted this project. Review what it would run now and approve again.")
    }
    public static var trustSummaryUntrusted: String {
        tr("trust-summary-untrusted", "It wants to run these commands. Nothing runs until you approve.")
    }
    public static var trustNothing: String { tr("trust-nothing", "Nothing here would run.") }
    public static var trustWaiting: String { tr("trust-waiting", "Waiting for approval…") }
    public static var trustWithTouchID: String { tr("trust-with-touch-id", "Trust with Touch ID") }
    public static var trustWithPassword: String { tr("trust-with-password", "Trust (approve with password)") }
    public static func trustLines(_ n: Int) -> String { plural(n, one: "1 line", other: "\(n) lines") }
}

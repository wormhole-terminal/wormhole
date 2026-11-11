// What VoiceOver is told beyond the accessibility tree: every announcement of the app goes through here, so the
// words are said one way (urgent ones interrupt, the rest wait their turn, as GPUI's `platform::a11y::announce`) and a
// script can read what was said (`a11y said`). A test instance keeps them to itself: it never speaks.

import AppKit

@MainActor
public enum Announcements {
    /// The last few things said, oldest first, `urgent` for the ones that interrupt (enough to see what a check caused).
    public private(set) static var recent: [(text: String, urgent: Bool)] = []
    /// Off in a background instance: the words are kept for `a11y said` and never posted.
    public static var speaks = true
    private static let keep = 16

    /// Says `text` through VoiceOver when it runs; `urgent` cuts into what it is saying (a terminal needs input).
    public static func post(_ text: String, urgent: Bool = false) {
        recent.append((text, urgent))
        if recent.count > keep { recent.removeFirst() }
        guard speaks else { return }
        NSAccessibility.post(
            element: NSApp as Any, notification: .announcementRequested,
            userInfo: [.announcement: text, .priority: (urgent ? NSAccessibilityPriorityLevel.high : .medium).rawValue])
    }
}

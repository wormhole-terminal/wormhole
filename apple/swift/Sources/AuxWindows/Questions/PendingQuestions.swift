// text-file: a test driver's replies on the control folder (a test instance only), not interface text
// A test instance (`--background`) never puts a question on the screen: presenting a sheet brings its window forward, over what
// the person is doing. The question is held here instead, and a driver answers it through the control folder (`question`). The
// real app has none of this (`current` stays nil): its questions are sheets, and nothing that arrives over the control socket
// can ever answer one.

import Foundation

@MainActor
public final class PendingQuestions {
    /// Set once, at launch, by a test instance.
    public static var current: PendingQuestions?

    private struct Held {
        var title: String
        var detail: String
        var buttons: [String]
        /// The buttons the words `confirm` and `cancel` press.
        var confirm: Int
        var cancel: Int
        var resume: CheckedContinuation<Int, Never>
    }

    private var held: [Held] = []

    public init() {}

    public var isEmpty: Bool { held.isEmpty }

    /// Waits for a driver to press a button; the answer is its place.
    func hold(title: String, detail: String, buttons: [String], confirm: Int, cancel: Int) async -> Int {
        await withCheckedContinuation { resume in
            held.append(Held(title: title, detail: detail, buttons: buttons, confirm: confirm, cancel: cancel, resume: resume))
        }
    }

    /// The oldest question, as a driver reads it.
    public var summary: String {
        guard let first = held.first else { return "no question is up" }
        let list = first.buttons.enumerated().map { "\($0.offset):\($0.element)" }.joined(separator: " | ")
        return "title=\(first.title)\ndetail=\(first.detail)\nbuttons=\(list) (confirm=\(first.confirm) cancel=\(first.cancel))\nwaiting=\(held.count)"
    }

    /// One line for the state dump.
    public var brief: String { held.first.map { "\($0.title) (\(held.count))" } ?? "none" }

    /// Presses a button of the oldest question: `confirm`, `cancel`, or `also [n]` (the nth extra button, from 1), or its place.
    public func answer(_ words: String) -> String {
        guard let first = held.first else { return "error: no question is up" }
        let parts = words.split(separator: " ").map(String.init)
        let place: Int?
        switch parts.first {
        case "confirm": place = first.confirm
        case "cancel": place = first.cancel
        case "also": place = (first.confirm + (parts.count > 1 ? Int(parts[1]) ?? 1 : 1))
        default: place = parts.first.flatMap { Int($0) }
        }
        guard let place, first.buttons.indices.contains(place) else { return "error: question confirm|cancel|also [n]|<button place>" }
        held.removeFirst().resume.resume(returning: place)
        return "question answered \(first.buttons[place])"
    }
}

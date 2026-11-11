// What the Look step has beyond the theme, the colour, the font and the ligatures (`look_step.rs`, `tailor_step.rs`):
// the offer to bring over another terminal's settings, and "How much to show", the walk through what the window
// displays. The walk, its words and the sketches' layout are the core's; this holds what a view needs of them.

import Foundation
import Observation

/// A block of a window sketch, in points from its top left; `fill` and `corner` say which colour and rounding.
public struct SketchBlock: Sendable, Equatable {
    /// What a block is made of, by role; the view maps each to a token.
    public enum Fill: Sendable { case sidebar, panelRight, rule, selectedRow, faint, muted, chip, primary, tab, minimap }
    /// How a block is rounded: not at all, by the theme's small radius, or as far as its thinner side allows.
    public enum Corner: Sendable { case square, soft, round }
    /// Where the block is, in points from the sketch's top left.
    public var x, y, w, h: Double
    /// Which colour of the theme it is drawn in.
    public var fill: Fill
    /// How it is rounded.
    public var corner: Corner

    /// A block at `x`, `y` of `w` by `h`.
    public init(x: Double, y: Double, w: Double, h: Double, fill: Fill, corner: Corner) {
        (self.x, self.y, self.w, self.h, self.fill, self.corner) = (x, y, w, h, fill, corner)
    }
}

/// Where the sketches of the first run's presets come from (the core lays them out): the app sets it at launch, and the
/// Studio's gallery asks it. Empty until then, which draws nothing.
public enum SketchSource {
    /// The blocks of preset `index` (0 Zen, 1 Balanced, 2 Everything) `width` points wide.
    @MainActor public static var preset: (_ index: Int, _ width: Double) -> [SketchBlock] = { _, _ in [] }
}

/// An answer of a question and the window it would give.
public struct TailorAnswer: Sendable, Equatable, Identifiable {
    /// The answer's name and the line about it.
    public var label, detail: String
    /// Whether the window shows this answer now.
    public var picked: Bool
    /// The window this answer would give, drawn small.
    public var sketch: [SketchBlock]
    /// The label names an answer within its question.
    public var id: String { label }

    /// An answer with its sketch.
    public init(label: String, detail: String, picked: Bool, sketch: [SketchBlock]) {
        (self.label, self.detail, self.picked, self.sketch) = (label, detail, picked, sketch)
    }
}

/// The walk as it is now: a question with its answers, the summary, or "All set".
public struct TailorView: Sendable, Equatable {
    /// Asking a question, the summary after the last, or "All set" after Looks good.
    public enum Stage: Sendable { case asking, summary, settled }
    /// Where the walk is.
    public var stage: Stage
    /// The words: the section heading, the question and its place, the line about the answer in use, the summary's title and the settled line.
    public var heading, question, counter, detail, summaryTitle, settledText: String
    /// The links' and the button's words, and what a screen reader is told as the walk moves.
    public var backLabel, startOverLabel, looksGoodLabel, announcement: String
    /// The question's answers (empty on the summary).
    public var answers: [TailorAnswer]
    /// Each question with the answer it has now.
    public var summary: [(question: String, answer: String)]
    /// Whether there is a question before this one.
    public var canBack: Bool
    /// Whether the window's right panel is to be open.
    public var panelOpen: Bool

    /// A view of the walk from its parts.
    public init(
        stage: Stage, heading: String, question: String, counter: String, detail: String, summaryTitle: String,
        settledText: String, backLabel: String, startOverLabel: String, looksGoodLabel: String, announcement: String,
        answers: [TailorAnswer], summary: [(question: String, answer: String)], canBack: Bool, panelOpen: Bool
    ) {
        (self.stage, self.heading, self.question, self.counter, self.detail) = (stage, heading, question, counter, detail)
        (self.summaryTitle, self.settledText, self.backLabel, self.startOverLabel) = (summaryTitle, settledText, backLabel, startOverLabel)
        (self.looksGoodLabel, self.announcement, self.answers, self.summary) = (looksGoodLabel, announcement, answers, summary)
        (self.canBack, self.panelOpen) = (canBack, panelOpen)
    }

    /// Equal when they draw the same: the words follow from these.
    public static func == (lhs: TailorView, rhs: TailorView) -> Bool {
        lhs.stage == rhs.stage && lhs.question == rhs.question && lhs.answers == rhs.answers && lhs.counter == rhs.counter
            && lhs.panelOpen == rhs.panelOpen && lhs.summary.map(\.answer) == rhs.summary.map(\.answer)
    }
}

/// What the person does in the walk.
public enum TailorAct: Sendable, Equatable {
    case answer(Int), back, startOver, settle
}

/// The walk of one run, over the core: every answer is written as it is given.
@MainActor
public protocol TailorWalking: AnyObject {
    func view(width: Double) -> TailorView
    /// Whether the act changed anything.
    func act(_ act: TailorAct) -> Bool
}

/// A terminal with settings on this Mac the step offers to bring over, in the interface's words.
public struct ImportChoice: Sendable, Equatable {
    /// The terminal's stable name (`ghostty`).
    public var source: String
    /// The offer in the interface's words ("Use my Ghostty settings").
    public var offer: String
    /// An offer.
    public init(source: String, offer: String) { (self.source, self.offer) = (source, offer) }
}

/// The Look step's own state beyond `OnboardingPreferences`: the offer found, what taking it did, the walk.
@MainActor @Observable
public final class LookExtras {
    /// The terminal found, once looked for.
    public private(set) var choice: ImportChoice?
    /// What bringing the settings over did, or why it did not work.
    public private(set) var brought: String?
    /// Why bringing the settings over did not work.
    public private(set) var failed: String?
    /// The walk as it is now; nil until it began or when the build has none.
    public private(set) var walk: TailorView?
    @ObservationIgnored public let backend: any OnboardingBackend
    @ObservationIgnored private var tailor: (any TailorWalking)?
    @ObservationIgnored private var width = 118.0
    @ObservationIgnored private var scanned = false

    /// The Look step's extras over `backend`.
    public init(backend: any OnboardingBackend) { self.backend = backend }

    /// Looks for another terminal's settings (off the main thread: it reads their files) and begins the walk, once:
    /// the first draw of the step.
    public func begin(width: Double) async {
        self.width = width
        if tailor == nil, let made = backend.tailor() {
            tailor = made
            walk = made.view(width: width)
        }
        guard !scanned else { return }
        scanned = true
        choice = await backend.importOffer()
    }

    /// Does `act` in the walk and redraws it; the window's panel follows what it says.
    public func act(_ act: TailorAct) {
        guard let tailor, tailor.act(act) else { return }
        walk = tailor.view(width: width)
        if let walk { backend.panelOpen(walk.panelOpen) }
    }

    /// The answers' sketches are laid out again for a new width.
    public func resize(to width: Double) {
        guard abs(width - self.width) > 0.5, let tailor else { return }
        self.width = width
        walk = tailor.view(width: width)
    }

    /// Brings over the offered terminal's settings.
    public func useImport() async {
        guard let choice else { return }
        do {
            brought = try await backend.useImport(choice)
            failed = nil
        } catch {
            failed = "\(error)"
        }
    }

    /// The theme file import, when no terminal's settings were found.
    public func importThemeFile() async { _ = await backend.importThemeFile() }
}

// The borderless icon button (`buttons.rs` `IconButton`): a 28 pt hit with a face drawn inside it, the glyph muted until the
// pointer is over it, then `text.primary` on `band.hover`; while selected (a pressed toggle, an active filter) the selected
// tint with its text colour; faint and still while disabled. Its tooltip is its VoiceOver name without the key it ends in
// (`spoken_name`). It also owns what each title row, band and row drew for itself: the face sizes (`in_title_row`,
// `in_band`, `in_row`), the active filter's dot (`marked`), the keyboard ring round the face (`focus_ring`), a spoken name of
// its own (`spoken`) and a drawing of the caller's in the glyph's place, given the colour of the moment (`with_drawing`, the
// column toggles' `strip`).

import SwiftUI

public struct IconButton<Glyph: View>: View {
    /// The tooltip; VoiceOver hears it without its key, unless `spoken` names the button.
    public let label: String
    public let action: () -> Void
    private let glyph: (_ colour: Color, _ size: CGFloat) -> Glyph
    /// A pressed toggle or an active filter (`IconButton::marked` with `selected`): the selected tint and its text colour.
    public var isSelected = false
    var isToggle = false
    var face = IconButtonFace.control
    var motion: IconMotion?
    var showsDot = false
    var dotSurface: Color?
    var ringShown = false
    var spokenName: String?
    @State private var hovering = false
    @Environment(\.isEnabled) private var enabled
    @Environment(\.tokens) private var tokens
    private let run = IconRun()
    /// Where the view that asked for it was built, for inspect mode.
    let file: StaticString
    let line: UInt

    /// A drawing of the caller's in the glyph's place (a column toggle's strip, an app's own icon), drawn in `colour`: the
    /// button's glyph colour of the moment.
    public init(
        label: String, file: StaticString = #fileID, line: UInt = #line, action: @escaping () -> Void,
        @ViewBuilder glyph: @escaping (_ colour: Color) -> Glyph
    ) {
        (self.label, self.action, self.file, self.line) = (label, action, file, line)
        self.glyph = { colour, _ in glyph(colour) }
    }

    fileprivate init(
        label: String, motion: IconMotion?, file: StaticString, line: UInt, action: @escaping () -> Void,
        glyph: @escaping (Color, CGFloat) -> Glyph
    ) {
        (self.label, self.action, self.file, self.line, self.glyph, self.motion) = (label, action, file, line, glyph, motion)
    }

    /// This button as a toggle that is on while `on`: it takes the selected tint and says so to VoiceOver.
    public func selected(_ on: Bool) -> Self { with { $0.isSelected = on } }

    /// A real toggle (the find bar's options): VoiceOver hears AppKit's toggle button with its value, on or off (the role
    /// `Toggle` with `.button` style has: a checkbox that is a toggle), where `selected` is a state of a button.
    public func toggled(_ on: Bool) -> Self { with { ($0.isSelected, $0.isToggle) = (on, true) } }

    /// The title rows' 24 pt face with a 16 pt glyph (`in_title_row`).
    public func inTitleRow() -> Self { with { $0.face = .titleRow } }

    /// A band's +: the same 24 pt face and 16 pt glyph (`in_band`).
    public func inBand() -> Self { with { $0.face = .titleRow } }

    /// A row's hover action: a 22 pt face with a 14 pt glyph (`in_row`).
    public func inRow() -> Self { with { $0.face = .row } }

    /// What the glyph does to answer a click, in place of the one its icon has (the splits fill their pane); `nil`: none.
    public func answering(_ motion: IconMotion?) -> Self { with { $0.motion = motion } }

    /// The small primary dot of a filter that is on, or a wanted panel waiting for room (`marked`), ringed by `surface`.
    public func dot(_ shown: Bool, on surface: Color? = nil) -> Self { with { ($0.showsDot, $0.dotSurface) = (shown, surface) } }

    /// The keyboard focus ring round the face (`focus_ring`): outside it, a point off, the face's corners.
    public func focusRing(_ shown: Bool) -> Self { with { $0.ringShown = shown } }

    /// What VoiceOver calls the button where its tooltip alone would make a list of identical ones (`spoken`).
    /// `nil` keeps the tooltip's words.
    public func spoken(_ name: String?) -> Self { with { $0.spokenName = name } }

    private func with(_ change: (inout Self) -> Void) -> Self {
        var copy = self
        change(&copy)
        return copy
    }

    public var body: some View {
        let colour = !enabled ? tokens.textFaint : isSelected ? tokens.sidebarRowSelectedText : hovering ? tokens.textPrimary : tokens.textMuted
        Button {
            run.start()
            action()
        } label: {
            glyph(colour, face.glyph)
                .iconMotion(motion, run: run.count)
                .foregroundStyle(colour)
        }
        .buttonStyle(IconButtonStyle(
            face: face.size, hovering: hovering && enabled, selected: isSelected, dot: showsDot ? dotSurface ?? tokens.sidebarBackground : nil,
            ring: ringShown))
        .marked("icon.button", style: "hit hitMin · face control/titleControl/controlSmall · radius control · glyph glyph", file: file, line: line)  // text: inspect mode's note, in the scales' names
        .onHover { hovering = $0 }
        .tooltipName(label, spoken: spokenName)
        .accessibilityAddTraits(isSelected ? .isSelected : [])
        .modifier(ToggleSpeech(on: isToggle ? isSelected : nil, tooltip: label, spoken: spokenName) {
            run.start()
            action()
        })
    }
}

/// A toggle's accessibility: a real `Toggle` stands in for the button, so VoiceOver hears the toggle's role and its value
/// ("Match case, off, toggle button") and Press flips it, as the find bar's options were before they were drawn.
private struct ToggleSpeech: ViewModifier {
    /// The toggle's state; nil leaves a plain button alone.
    let on: Bool?
    let tooltip: String
    let spoken: String?
    let flip: () -> Void

    func body(content: Content) -> some View {
        if let on {
            let split = SpokenName.split(tooltip)
            content.accessibilityRepresentation {
                Toggle(isOn: Binding(get: { on }, set: { _ in flip() })) { Text(spoken ?? split.words) }
                    .toggleStyle(.button)
                    .accessibilityHint(split.key.map(KeyCapSpeech.words) ?? "")
            }
        } else {
            content
        }
    }
}

/// The plain glyph of an icon button: a Lucide drawing where GPUI's is generated, else an SF Symbol.
public struct IconButtonGlyph: View {
    let symbol: String
    let lucide: Lucide?
    let size: CGFloat

    public var body: some View {
        if let lucide { LucideView(icon: lucide, size: size) } else { Image(systemName: symbol).font(.system(size: size, weight: .medium)) }
    }
}

extension IconButton where Glyph == IconButtonGlyph {
    public init(symbol: String, label: String, file: StaticString = #fileID, line: UInt = #line, action: @escaping () -> Void) {
        self.init(label: label, motion: IconMotion.of(symbol: symbol), file: file, line: line, action: action) { _, size in
            IconButtonGlyph(symbol: symbol, lucide: nil, size: size)
        }
    }

    /// The Lucide glyph GPUI's button draws.
    public init(lucide: Lucide, label: String, file: StaticString = #fileID, line: UInt = #line, action: @escaping () -> Void) {
        self.init(label: label, motion: IconMotion.of(lucide), file: file, line: line, action: action) { _, size in
            IconButtonGlyph(symbol: "", lucide: lucide, size: size)
        }
    }
}

/// The drawn face inside the 28 pt hit, and its glyph (`buttons.rs`: `HIT_MIN`, `BAND_BUTTON`, `ROW_BUTTON`).
enum IconButtonFace {
    case control, titleRow, row

    var size: CGFloat {
        switch self {
        case .control: Metric.control
        case .titleRow: Metric.titleControl
        case .row: Metric.controlSmall
        }
    }

    var glyph: CGFloat { self == .titleRow ? Metric.titleGlyph : Metric.glyph }
}

private struct IconButtonStyle: ButtonStyle {
    let face: CGFloat
    let hovering: Bool
    let selected: Bool
    /// The surface the dot's ring is cut from, while the dot shows.
    let dot: Color?
    let ring: Bool
    @Environment(\.tokens) private var tokens
    @Environment(\.isEnabled) private var enabled

    func makeBody(configuration: Configuration) -> some View {
        let radius = tokens.metrics.radius
        let shape = RoundedRectangle(cornerRadius: radius)
        // Nothing to do now (an Undo with no step): no hover and no press (`IconButton::disabled`).
        let pressed = enabled && configuration.isPressed
        let fill = selected && !pressed ? tokens.stateSelected : pressed ? tokens.stateActive : hovering ? tokens.bandHover : .clear
        return configuration.label
            .frame(width: face, height: face)
            .background(fill, in: shape)
            .overlay(alignment: .topTrailing) { if let dot { ActiveDot(surface: dot) } }
            .keyboardFocusRing(ring, radius: radius, outset: IconButtonMark.ringOffset + tokens.metrics.focusRingWidth)
            .frame(width: Metric.hitMin, height: Metric.hitMin)
            .contentShape(Rectangle())
            // Hover has no animation, as GPUI's `group_hover`; only the press sinks.
            .pressed(pressed)
    }
}

/// The active filter's dot (`buttons.rs` `mark`): the primary, ringed by the surface it sits on, near the face's top right.
private struct ActiveDot: View {
    let surface: Color
    @Environment(\.tokens) private var tokens

    var body: some View {
        let inset = IconButtonMark.inset - IconButtonMark.ring
        Circle().fill(tokens.primary).frame(width: IconButtonMark.dot, height: IconButtonMark.dot)
            .padding(IconButtonMark.ring).background(surface, in: Circle())
            .offset(x: -inset, y: inset)
            .accessibilityHidden(true)
    }
}

/// `buttons.rs`: `MARK_INSET`, `MARK_RING`, `NARROW_GLYPH` (a menu's chevron beside a button); `scale.rs`: `MARK_DOT`,
/// `FOCUS_RING_OFFSET`.
enum IconButtonMark {
    static let inset: CGFloat = 5
    static let ring: CGFloat = 2
    static let dot: CGFloat = 6
    static let ringOffset: CGFloat = 1
    static let narrowGlyph: CGFloat = 12
}

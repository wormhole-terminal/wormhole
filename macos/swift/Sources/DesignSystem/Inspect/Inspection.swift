// Inspect mode's ground (the GPUI app's `inspector/`): an element of the design says where it was built and what it is
// drawn with (`InspectSite`, which `.marked` makes), and while inspect mode is on each one puts an empty view behind
// itself, so the inspector can ask which elements lie under a point and where. Off, nothing is added: a marked view reads
// one flag. The panel, the picking and the note are the app's (`Wormhole/Inspect`).

import AppKit
import Observation
import SwiftUI

/// Where an element was built and how it is styled, in the design system's terms.
public struct InspectSite: Sendable {
    /// The component's id (core's `TOKEN_USES` id where the Studio's gallery knows it).
    public let id: String
    /// `#fileID` and `#line` of the view that asked for it (a shared component reports its caller, not itself).
    public let file: StaticString
    public let line: UInt
    /// Its spacing, radius and text as the scales name them (`padding x _1 · radius chip · text caption`); empty when
    /// the element does not say.
    public let style: String

    /// A site for the element `id` built at `file`:`line`.
    public init(id: String, style: String = "", file: StaticString = #fileID, line: UInt = #line) {
        (self.id, self.style, self.file, self.line) = (id, style, file, line)
    }

    /// `SidebarUI/RowView.swift:42`
    public var location: String { "\(file):\(line)" }
}

/// An element found under a point: its site and its frame in its window (AppKit's window coordinates).
public struct InspectedElement {
    /// Where it was built.
    public let site: InspectSite
    /// Its frame in the window, from the bottom left.
    public let frame: CGRect
}

/// Whether inspect mode is on, and the elements that reported themselves while it is.
@MainActor @Observable
public final class Inspection {
    /// The one inspect mode of the app.
    public static let shared = Inspection()

    /// On: every marked element keeps a probe behind itself. Off (always, unless the inspector turned it on): none does.
    public var isOn = false

    @ObservationIgnored private let probes = NSHashTable<InspectProbeView>.weakObjects()

    func add(_ probe: InspectProbeView) { probes.add(probe) }

    /// The elements of `window` under `point` (window coordinates), innermost first: the smallest is what was meant, and
    /// the ones after it are what it stands in, as GPUI's wheel walks out to the parents.
    public func elements(at point: NSPoint, in window: NSWindow) -> [InspectedElement] {
        probes.allObjects
            .filter { $0.window === window && !$0.isHiddenOrHasHiddenAncestor }
            .compactMap { probe in probe.site.map { InspectedElement(site: $0, frame: probe.convert(probe.bounds, to: nil)) } }
            .filter { $0.frame.contains(point) }
            .sorted { $0.frame.width * $0.frame.height < $1.frame.width * $1.frame.height }
    }
}

/// The empty view behind a marked element while inspect mode is on: it is where the element is.
final class InspectProbeView: NSView {
    var site: InspectSite?

    // Never in the pointer's way.
    override func hitTest(_ point: NSPoint) -> NSView? { nil }
}

struct InspectProbe: NSViewRepresentable {
    let site: InspectSite

    func makeNSView(context: Context) -> InspectProbeView {
        let view = InspectProbeView()
        view.site = site
        Inspection.shared.add(view)
        return view
    }

    func updateNSView(_ view: InspectProbeView, context: Context) { view.site = site }
}

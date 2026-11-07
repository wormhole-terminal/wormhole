// An agent's logo, filled in the foreground colour in its 24 point box (the registry's `agent-icons/*.svg`). Agents
// without one (Aider, Crush, Factory Droid, Auggie, Foundation Models) show their monogram instead.

import SwiftUI
import WorkspaceModel

struct AgentLogo: View {
    let agent: AgentID
    let size: CGFloat
    private static let box: CGFloat = 24

    /// The file the registry names the logo by; nil for an agent that has none.
    static func key(_ agent: AgentID) -> String? {
        let key = agent == .openCode ? "opencode" : agent.rawValue
        return drawings[key] == nil ? nil : key
    }

    var body: some View {
        Canvas { canvas, area in
            guard let key = Self.key(agent), let drawing = Self.drawings[key] else { return }
            let scale = area.width / Self.box
            var path = Path()
            for d in drawing.paths { path.addPath(SVGPath.path(d), transform: CGAffineTransform(scaleX: scale, y: scale)) }
            canvas.fill(path, with: .foreground, style: FillStyle(eoFill: drawing.evenOdd))
        }
        .frame(width: size, height: size)
    }
}

// What an agent's tile is painted with and says inside it, from the agent registry (`wormhole-term` `agents/mod.rs`
// `Agent::tile` and `monogram`). A brand's colour is the brand's, not the theme's: the tile is the agent's app icon and
// looks the same in every theme, so the colours sit here beside the registry's source for each, never in a token.
// What the theme decides stays a token: the black tile (`tile.background`) and the mark on a brand's colour.

import SwiftUI
import WorkspaceModel

enum TilePaint: Equatable {
    /// The theme's black tile, for a brand that is black and white.
    case black
    /// One colour, `0xRRGGBB`.
    case solid(UInt32)
    /// From the top left to the bottom right.
    case gradient(UInt32, UInt32)
}

extension AgentID {
    var tilePaint: TilePaint {
        switch self {
        case .claude: .solid(0xd97757)  // Lobe Icons' and Simple Icons' Claude orange
        case .codex: .solid(0x3941ff)  // OpenAI Codex's own blue; its lighter stops are too light for the white logo
        case .gemini: .solid(0x3186ff)  // Gemini's blue
        case .aider: .solid(0x014f2c)
        case .qwen: .solid(0x082dff)
        case .crush: .solid(0x6b50ff)
        case .kiro: .solid(0x9046ff)  // kiro.dev's own icon, with a white ghost on it
        case .copilot, .cursor, .openCode, .amp, .goose, .droid, .auggie, .cline, .pi, .fm: .black
        }
    }

    /// Two capitals: the initials of the name's words, or the first two letters, unless the registry fixes them.
    var monogram: String {
        switch self {
        case .claude: "CC"
        case .codex: "CX"
        case .gemini: "GM"
        case .copilot: "GC"
        case .cursor: "CA"
        case .openCode: "OC"
        case .aider: "AI"
        case .amp: "AM"
        case .goose: "GO"
        case .qwen: "QC"
        case .crush: "CR"
        case .droid: "FD"
        case .auggie: "AU"
        case .kiro: "KI"
        case .cline: "CL"
        case .pi: "PI"
        case .fm: "FM"
        }
    }
}

extension TilePaint {
    func shape<S: InsettableShape>(_ shape: S, black: Color) -> some View {
        Group {
            switch self {
            case .black: shape.fill(black)
            case .solid(let rgb): shape.fill(Color(rgb: rgb))
            case .gradient(let from, let to):
                shape.fill(LinearGradient(colors: [Color(rgb: from), Color(rgb: to)], startPoint: .topLeading, endPoint: .bottomTrailing))
            }
        }
    }
}

extension Color {
    init(rgb: UInt32) {
        self.init(.sRGB, red: Double(rgb >> 16 & 0xff) / 255, green: Double(rgb >> 8 & 0xff) / 255, blue: Double(rgb & 0xff) / 255)
    }
}

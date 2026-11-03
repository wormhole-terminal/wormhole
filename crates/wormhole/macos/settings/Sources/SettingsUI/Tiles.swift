import SwiftUI

/// A coloured rounded square with a white symbol, as in System Settings: the symbol sits centred with even padding.
/// Sidebar items, rows (agents, projects, probes, import) and the Changed page all draw this one view.
struct Tile: View {
    let colour: Color; let symbol: String
    static func symbol(for icon: String?) -> String? { ["claude": "asterisk", "prompt": "chevron.right", "spark": "sparkle"][icon ?? ""] }
    var body: some View {
        RoundedRectangle(cornerRadius: TileSize.radius, style: .continuous).fill(colour)
            .frame(width: TileSize.side, height: TileSize.side)
            .overlay(Image(systemName: symbol).font(.system(size: TileSize.symbol, weight: .medium)).foregroundStyle(.white)
                .minimumScaleFactor(0.5).lineLimit(1).padding(TileSize.inset))
    }
}

/// The profile's avatar: a circle of the same side as a tile, with the profile's letter.
struct Avatar: View {
    let letter: String; let fill: Color; let ink: Color
    var body: some View {
        Circle().fill(fill).frame(width: TileSize.side, height: TileSize.side)
            .overlay(Text(letter).font(.system(size: TileSize.letter, weight: .semibold)).foregroundStyle(ink))
    }
}

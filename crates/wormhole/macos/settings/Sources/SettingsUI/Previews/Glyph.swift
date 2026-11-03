import SwiftUI

/// A 16×16 line icon from an SVG path string (M, L, H, V, l, h, v, z only), drawn like the board's icons.
struct Glyph: Shape {
    let d: String
    func path(in r: CGRect) -> Path {
        var p = Path(); var cur = CGPoint.zero; var start = CGPoint.zero
        let s = r.width / 16
        func pt(_ x: CGFloat, _ y: CGFloat) -> CGPoint { CGPoint(x: r.minX + x * s, y: r.minY + y * s) }
        var tokens: [String] = []; var buf = ""
        for ch in d {
            if ch.isLetter { if !buf.isEmpty { tokens.append(buf); buf = "" }; tokens.append(String(ch)) }
            else if ch == "-" { if !buf.isEmpty { tokens.append(buf) }; buf = "-" }
            else if ch == " " || ch == "," { if !buf.isEmpty { tokens.append(buf); buf = "" } }
            else { buf.append(ch) }
        }
        if !buf.isEmpty { tokens.append(buf) }
        var i = 0; var cmd = "M"; var x: CGFloat = 0, y: CGFloat = 0
        func num() -> CGFloat { defer { i += 1 }; return CGFloat(Double(tokens[i]) ?? 0) }
        while i < tokens.count {
            if tokens[i].first!.isLetter { cmd = tokens[i]; i += 1; if cmd == "z" { p.closeSubpath(); cur = start; (x, y) = (cur.x, cur.y); continue } }
            switch cmd {
            case "M": (x, y) = (num(), num()); p.move(to: pt(x, y)); start = CGPoint(x: x, y: y); cmd = "L"
            case "m": (x, y) = (x + num(), y + num()); p.move(to: pt(x, y)); start = CGPoint(x: x, y: y); cmd = "l"
            case "L": (x, y) = (num(), num()); p.addLine(to: pt(x, y))
            case "l": (x, y) = (x + num(), y + num()); p.addLine(to: pt(x, y))
            case "H": x = num(); p.addLine(to: pt(x, y))
            case "h": x += num(); p.addLine(to: pt(x, y))
            case "V": y = num(); p.addLine(to: pt(x, y))
            case "v": y += num(); p.addLine(to: pt(x, y))
            default: i += 1
            }
        }
        return p
    }
}

enum GlyphPath {
    static let claude = "M8 2v12M2 8h12M3.8 3.8l8.4 8.4M12.2 3.8l-8.4 8.4"
    static let prompt = "M4 5l3 3-3 3M8.5 11.5H12"
    static let spark = "M8 2.5l1.4 4.1 4.1 1.4-4.1 1.4L8 13.5l-1.4-4.1L2.5 8l4.1-1.4z"
    static let notes = "M4 2.5h8v11H4zM6 6h4M6 9h4"
}

struct GlyphIcon: View {
    let d: String; var size: CGFloat = 12; var colour: Color = .white; var line: CGFloat = 1.8
    var body: some View {
        Glyph(d: d).stroke(colour, style: StrokeStyle(lineWidth: line * size / 16, lineCap: .round, lineJoin: .round)).frame(width: size, height: size)
    }
}

// How a theme draws a folder's eight colour names (`wormhole_core::theme::folder_color`): the hue of the name, fixed so a
// "Teal" is teal in every theme, at the lightness and chroma of the theme's own six ANSI colours, so a pastel theme gets
// pastels and a light one darker colours. The core's last step (moving it far enough from the window and the sidebar to
// be seen on both) is the FFI's; until the theme dump carries these colours this is the stand-in, without that step.

import DesignSystem
import SwiftUI
import WorkspaceModel

enum FolderTint {
    static func color(_ name: FolderColor, in tokens: ThemeTokens) -> Color {
        let (lightness, chroma) = character(of: tokens)
        return oklch(lightness, chroma, hue(name))
    }

    /// Where on the colour wheel (OKLCH hue, degrees) the name is.
    static func hue(_ name: FolderColor) -> Double {
        switch name {
        case .red: 25
        case .orange: 55
        case .yellow: 95
        case .green: 145
        case .teal: 190
        case .blue: 255
        case .purple: 305
        case .pink: 350
        }
    }

    /// The average OKLCH lightness and chroma of ANSI 1 to 6 (red to cyan), the chroma at least 0.08 so two names of a
    /// nearly grey palette can still be told apart.
    private static func character(of tokens: ThemeTokens) -> (Double, Double) {
        let samples = tokens.terminal.ansi[1...6].map(lightnessChroma)
        let count = Double(samples.count)
        return (samples.map(\.0).reduce(0, +) / count, max(samples.map(\.1).reduce(0, +) / count, 0.08))
    }

    private static func lightnessChroma(_ c: RGBA) -> (Double, Double) {
        let (l, m, s) = lms(linear(c.red), linear(c.green), linear(c.blue))
        let (L, a, b) = (0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s, 1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s, 0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s)
        return (L, (a * a + b * b).squareRoot())
    }

    private static func lms(_ r: Double, _ g: Double, _ b: Double) -> (Double, Double, Double) {
        (
            cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b), cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b),
            cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b)
        )
    }

    private static func linear(_ v: Double) -> Double { v <= 0.04045 ? v / 12.92 : pow((v + 0.055) / 1.055, 2.4) }
    private static func gamma(_ v: Double) -> Double { v <= 0.0031308 ? 12.92 * v : 1.055 * pow(v, 1 / 2.4) - 0.055 }

    private static func oklch(_ L: Double, _ C: Double, _ hue: Double) -> Color {
        let a = C * cos(hue * .pi / 180), b = C * sin(hue * .pi / 180)
        let (l, m, s) = (pow(L + 0.3963377774 * a + 0.2158037573 * b, 3), pow(L - 0.1055613458 * a - 0.0638541728 * b, 3), pow(L - 0.0894841775 * a - 1.2914855480 * b, 3))
        let channels = [4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s, -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s, -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s]
        let c = channels.map { min(max(gamma($0), 0), 1) }
        return Color(.sRGB, red: c[0], green: c[1], blue: c[2])
    }
}

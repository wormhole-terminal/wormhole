// The colour maths a folder's or a group's colour is worked out with, ported from `wormhole-core`
// `theme/tokens/color.rs` (`luminance`, `contrast_ratio`, `lighten`, `readable_on`, `to_oklch`, `from_oklch`): OKLab
// so equal steps look equal across hues, and out-of-gamut colours shed chroma rather than shift hue.

import DesignSystem
import Foundation

extension RGBA {
    /// WCAG 2.x relative luminance (alpha ignored; composite first).
    var luminance: Double {
        0.2126 * Self.linear(red) + 0.7152 * Self.linear(green) + 0.0722 * Self.linear(blue)
    }

    /// WCAG contrast ratio, 1...21, symmetric. Alpha is ignored, so composite translucent colours with `over` first.
    public static func contrast(_ a: RGBA, _ b: RGBA) -> Double {
        let (la, lb) = (a.luminance, b.luminance)
        return (max(la, lb) + 0.05) / (min(la, lb) + 0.05)
    }

    func withAlpha(_ alpha: Double) -> RGBA { RGBA(red, green, blue, min(max(alpha, 0), 1)) }

    /// OKLab `(L, a, b)`.
    var oklab: (l: Double, a: Double, b: Double) {
        let (r, g, b) = (Self.linear(red), Self.linear(green), Self.linear(blue))
        let l = cbrt(0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b)
        let m = cbrt(0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b)
        let s = cbrt(0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b)
        return (
            0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
            1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
            0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s
        )
    }

    /// OKLCH `(L, chroma, hue in degrees 0..<360)`: OKLab's polar form, where a hue is a number.
    var oklch: (l: Double, chroma: Double, hue: Double) {
        let lab = oklab
        let hue = atan2(lab.b, lab.a) * 180 / .pi
        return (lab.l, hypot(lab.a, lab.b), hue < 0 ? hue + 360 : hue)
    }

    /// The opaque colour of OKLCH `(l, chroma, hue)`; one the sRGB gamut can't hold loses chroma, keeping its
    /// lightness and hue.
    static func oklch(_ l: Double, _ chroma: Double, _ hue: Double) -> RGBA {
        let radians = hue * .pi / 180
        return fromOKLab(l, chroma * cos(radians), chroma * sin(radians), alpha: 1)
    }

    /// Raises OKLab lightness by `t` (absolute, lightness is 0...1); a negative `t` lowers it.
    func lightened(_ t: Double) -> RGBA {
        let lab = oklab
        return Self.fromOKLab(min(max(lab.l + t, 0), 1), lab.a, lab.b, alpha: alpha)
    }

    /// `self` moved along OKLab lightness toward white or black (whichever contrasts better with `background`) just far
    /// enough to reach `ratio` against it. Keeps alpha; contrast is measured on the composite.
    func readable(on background: RGBA, ratio: Double) -> RGBA {
        let ratioOf = { (c: RGBA) in RGBA.contrast(c.over(background), background) }
        if ratioOf(self) >= ratio { return self }
        let towardWhite = Self.contrast(RGBA(1, 1, 1), background) >= Self.contrast(RGBA(0, 0, 0), background)
        let step = { (t: Double) in towardWhite ? self.lightened(t) : self.lightened(-t) }
        if ratioOf(step(1)) < ratio { return step(1) }
        // Lightness against contrast is monotonic, so bisect for the smallest move.
        var (lo, hi) = (0.0, 1.0)
        for _ in 0..<24 {
            let mid = (lo + hi) / 2
            if ratioOf(step(mid)) >= ratio { hi = mid } else { lo = mid }
        }
        return step(hi)
    }

    private static func linear(_ v: Double) -> Double { v <= 0.04045 ? v / 12.92 : pow((v + 0.055) / 1.055, 2.4) }

    private static func gamma(_ v: Double) -> Double {
        let v = min(max(v, 0), 1)
        return v <= 0.003_130_8 ? v * 12.92 : 1.055 * pow(v, 1 / 2.4) - 0.055
    }

    private static func fromOKLab(_ l: Double, _ a: Double, _ b: Double, alpha: Double) -> RGBA {
        func raw(_ a: Double, _ b: Double) -> (Double, Double, Double) {
            let l_ = pow(l + 0.396_337_78 * a + 0.215_803_76 * b, 3)
            let m_ = pow(l - 0.105_561_346 * a - 0.063_854_17 * b, 3)
            let s_ = pow(l - 0.089_484_18 * a - 1.291_485_5 * b, 3)
            return (
                4.076_741_7 * l_ - 3.307_711_6 * m_ + 0.230_969_94 * s_,
                -1.268_438 * l_ + 2.609_757_4 * m_ - 0.341_319_38 * s_,
                -0.004_196_086_3 * l_ - 0.703_418_6 * m_ + 1.707_614_7 * s_
            )
        }
        func inGamut(_ c: (Double, Double, Double)) -> Bool {
            [c.0, c.1, c.2].allSatisfy { (-1e-4...(1 + 1e-4)).contains($0) }
        }
        // Chroma must vanish at the extremes, or rounding leaves a faint tint on "black" and "white".
        if l <= 0 { return RGBA(0, 0, 0, alpha) }
        if l >= 1 { return RGBA(1, 1, 1, alpha) }
        var rgb = raw(a, b)
        if !inGamut(rgb) {
            var (lo, hi) = (0.0, 1.0)
            for _ in 0..<20 {
                let mid = (lo + hi) / 2
                if inGamut(raw(a * mid, b * mid)) { lo = mid } else { hi = mid }
            }
            rgb = raw(a * lo, b * lo)
        }
        return RGBA(gamma(rgb.0), gamma(rgb.1), gamma(rgb.2), alpha)
    }
}

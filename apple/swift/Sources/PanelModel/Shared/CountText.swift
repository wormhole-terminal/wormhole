// How the panel writes numbers: a band's badge groups thousands ("1,234"), a tab's badge caps ("999+").
// `wormhole_core::view::limits`.

public enum CountText {
    public static let cap = 999

    public static func grouped(_ number: Int) -> String {
        let digits = String(number)
        var out = ""
        for (index, digit) in digits.enumerated() {
            if index > 0, (digits.count - index) % 3 == 0 { out.append(",") }
            out.append(digit)
        }
        return out
    }

    public static func capped(_ number: Int) -> String { number > cap ? "\(cap)+" : String(number) }
}

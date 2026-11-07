// The Changes rows cut into cards: each band starts one, the rows after it go into it, the spacer stays out.

import PanelModel

struct ChangeSection: Identifiable {
    let band: ChangeRow
    var rows: [ChangeRow] = []
    var id: String { band.id }

    static func cut(_ rows: [ChangeRow]) -> [ChangeSection] {
        var sections: [ChangeSection] = []
        for row in rows {
            switch row {
            case .band: sections.append(ChangeSection(band: row))
            case .spacer: break
            default: if !sections.isEmpty { sections[sections.count - 1].rows.append(row) }
            }
        }
        return sections
    }
}

//! The name of an SGR sequence: what each of its numbers sets.

use wormhole_vt::parser::Csi;

pub(crate) const COLORS: [&str; 8] = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];

pub(crate) fn sgr_text(csi: &Csi) -> String {
    let groups: Vec<&[u16]> = csi.groups().collect();
    if groups.is_empty() || (groups.len() == 1 && csi.get(0).is_none()) {
        return "reset".into();
    }
    let mut parts = Vec::new();
    let mut k = 0;
    while k < groups.len() {
        let g = groups[k];
        k += 1;
        let p = g[0];
        let part = match p {
            0 => "reset".to_string(),
            1 => "bold".into(),
            2 => "faint".into(),
            3 => "italic".into(),
            4 => match g.get(1) {
                None | Some(1) => "underline".into(),
                Some(0) => "no underline".into(),
                Some(2) => "double underline".into(),
                Some(3) => "curly underline".into(),
                Some(4) => "dotted underline".into(),
                Some(5) => "dashed underline".into(),
                Some(_) => "underline style?".into(),
            },
            5 | 6 => "blink".into(),
            7 => "reverse".into(),
            8 => "hidden".into(),
            9 => "strikethrough".into(),
            21 => "double underline".into(),
            22 => "normal intensity".into(),
            23 => "no italic".into(),
            24 => "no underline".into(),
            25 => "no blink".into(),
            27 => "no reverse".into(),
            28 => "no hidden".into(),
            29 => "no strikethrough".into(),
            30..=37 => format!("fg {}", COLORS[(p - 30) as usize]),
            39 => "default fg".into(),
            40..=47 => format!("bg {}", COLORS[(p - 40) as usize]),
            49 => "default bg".into(),
            53 => "overline".into(),
            55 => "no overline".into(),
            59 => "default underline color".into(),
            90..=97 => format!("fg bright {}", COLORS[(p - 90) as usize]),
            100..=107 => format!("bg bright {}", COLORS[(p - 100) as usize]),
            38 | 48 | 58 => {
                let what = match p {
                    38 => "fg",
                    48 => "bg",
                    _ => "underline color",
                };
                // `38:5:n` and `38:2::r:g:b` are one group; `38;5;n` and `38;2;r;g;b` are several.
                let spec: Vec<u16> = if g.len() > 1 {
                    g[1..].to_vec()
                } else {
                    let mut rest = Vec::new();
                    let wanted = match groups.get(k).map(|g| g[0]) {
                        Some(5) => 2,
                        Some(2) => 4,
                        _ => 0,
                    };
                    for _ in 0..wanted {
                        if let Some(next) = groups.get(k) {
                            rest.push(next[0]);
                            k += 1;
                        }
                    }
                    rest
                };
                match spec.as_slice() {
                    [5, n] => format!("{what} palette {n}"),
                    [2, .., r, g, b] => format!("{what} rgb({r},{g},{b})"),
                    _ => format!("{what} ?"),
                }
            }
            _ => format!("unknown {p}"),
        };
        parts.push(part);
    }
    if parts.len() > 10 {
        parts.truncate(10);
        parts.push("…".into());
    }
    parts.join(", ")
}

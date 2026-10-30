//! What devctl's `state` says of a picker: the text, the ghost, the crumbs, the lines with the selection
//! marked, what each row carries (badge, chevron, detail, what ↩ inserts and what ⇥ goes to), and where the
//! panel is. A native panel is not in GPUI's screenshots, so this is how a scripted check sees it.

use wormhole_core::view::picker::Item;

use super::Picker;

/// The picker as plain lines of text.
pub(crate) fn summary(picker: &Picker) -> String {
    let s = picker.model.snapshot();
    let items = picker.model.items();
    let mut out = format!(
        "typed {:?}\nghost {:?}\nbusy {}\ncrumbs {}\nfacts up {} descends {} ghost {} forgettable {}\nlines {} top {} shown {}\n",
        s.text,
        s.ghost.clone().unwrap_or_default(),
        s.busy,
        s.crumbs.iter().map(|c| c.label.as_str()).collect::<Vec<_>>().join(" > "),
        s.facts.up,
        s.facts.descends,
        s.facts.ghost,
        s.facts.forgettable,
        s.lines,
        s.top,
        s.items.len(),
    );
    for (at, item) in items.iter().enumerate() {
        let mark = if s.selected == Some(at) { '>' } else { ' ' };
        match item {
            Item::Header(title) => out.push_str(&format!("{mark} # {title}\n")),
            Item::Note(note) => out.push_str(&format!("{mark} note {note:?}\n")),
            Item::Row(row) => {
                out.push_str(&format!("{mark} {} [{}]", row.label, row.icon));
                if let Some(badge) = &row.badge {
                    out.push_str(&format!(" badge {badge:?}"));
                }
                if row.chevron {
                    out.push_str(" chevron");
                }
                if let Some(detail) = &row.detail {
                    out.push_str(&format!(" detail {detail:?}"));
                }
                if row.forgettable {
                    out.push_str(" forgettable");
                }
                if !row.marks.is_empty() {
                    out.push_str(&format!(" marks {:?}", row.marks));
                }
                out.push_str(&format!(
                    " insert {:?} descend {:?} key {:?}\n",
                    row.insert.as_deref().unwrap_or(""),
                    row.descend.as_deref().unwrap_or(""),
                    row.key
                ));
            }
        }
    }
    match picker.placed() {
        Some(p) => out.push_str(&format!(
            "panel x {:.0} y {:.0} w {:.0} h {:.0} {} rows {}\n",
            p.rect.x,
            p.rect.y,
            p.rect.w,
            p.rect.h,
            if p.above { "above" } else { "below" },
            p.rows,
        )),
        None => out.push_str("panel not placed yet\n"),
    }
    match picker.panel_info() {
        Some(info) => out.push_str(&format!(
            "native {} key {} visible {} window {} parent {} frame {:?}\nnative rows {:?}\nnative row fonts {:?}\nnative ghost {:?}\nnative crumbs {:?}\nnative segments {:?}\n",
            info.material.name(),
            info.key,
            info.visible,
            info.window_number,
            info.parent_number,
            info.frame,
            info.rows,
            info.row_fonts,
            info.ghost,
            info.crumbs,
            info.segments,
        )),
        None => out.push_str("native panel none\n"),
    }
    out
}

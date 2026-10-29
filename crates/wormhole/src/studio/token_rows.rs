//! What the token list shows, as rows: the bands and tokens flattened into one list so that the
//! list can draw the ones in view, with what each needs to draw itself worked out here, once per
//! change rather than once per frame. Pure, so it is tested without a window.

use std::collections::BTreeSet;

use wormhole_core::theme::tokens::{self, Color, ThemeFile, ThemeTokens};

use super::document::places::{self, Origin, Reach};
use super::document::{is_metric, number, override_of};
use super::draft::{Show, group_size, in_area, visible};
use crate::theme_bridge::MISSING_TOKEN;

/// What a token row's swatch draws.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Chip {
    Color(Color),
    /// A metric: a corner or an outline of that many points.
    Metric(f32),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Band {
    pub(super) name: &'static str,
    pub(super) open: bool,
    /// The band can be folded and opened (not while a filter lists what it found).
    pub(super) toggles: bool,
    pub(super) changed: bool,
    /// How many of its tokens fall short of a contrast minimum.
    pub(super) low: usize,
    /// The group's size in the area, also while a filter lists fewer of it.
    pub(super) size: usize,
    /// The band above is folded too: they share one hairline, as in the sidebar.
    pub(super) joined: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct TokenRow {
    pub(super) name: String,
    pub(super) chip: Chip,
    /// What the draft says itself, as typed; otherwise the value it resolves to.
    pub(super) value: String,
    pub(super) selected: bool,
    pub(super) low: bool,
    /// Edited since the last save.
    pub(super) changed: bool,
    /// Where the value comes from.
    pub(super) origin: Origin,
    /// For a pair of appearances: which of them the value is for.
    pub(super) reach: Option<Reach>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum ListRow {
    /// The token being edited stays in the editor when the area leaves it out: the list says why
    /// it doesn't show it rather than dropping it silently.
    NotInArea(String),
    Band(Band),
    Token(TokenRow),
}

impl ListRow {
    /// Whether `other` is the same line of the list, however its contents changed: the list keeps
    /// the heights it measured (and the scroll position) of lines that are.
    pub(super) fn same_line(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::NotInArea(_), Self::NotInArea(_)) => true,
            (Self::Band(a), Self::Band(b)) => a.name == b.name,
            (Self::Token(a), Self::Token(b)) => a.name == b.name,
            _ => false,
        }
    }
}

/// Everything the rows are made from.
pub(super) struct Inputs<'a> {
    pub(super) draft: &'a ThemeFile,
    pub(super) initial: &'a ThemeFile,
    /// The appearance being edited, while the theme is a pair.
    pub(super) shown: Option<&'a str>,
    pub(super) tokens: &'a ThemeTokens,
    /// The tokens whose text falls short somewhere.
    pub(super) low: &'a BTreeSet<String>,
    pub(super) show: Show,
    /// The part of the app the gallery shows (`None`: all of it) and its name for the note.
    pub(super) area: Option<tokens::Area>,
    pub(super) area_label: &'a str,
    /// The filter text.
    pub(super) query: &'a str,
    pub(super) open_groups: &'a BTreeSet<&'static str>,
    pub(super) selected: Option<&'a str>,
}

pub(super) struct Built {
    pub(super) rows: Vec<ListRow>,
    /// The counts the Yours, Base and Low contrast segments carry, in the area.
    pub(super) yours_count: usize,
    pub(super) base_count: usize,
    pub(super) low_count: usize,
}

pub(super) fn build(inputs: &Inputs) -> Built {
    let Inputs { draft, initial, tokens, low, show, area, query, .. } = *inputs;
    let changed = |name: &str| override_of(draft, name) != override_of(initial, name);
    let pair = places::is_pair(draft);
    let origin = |name: &str| places::origin(draft, inputs.shown, name);
    let shown = visible(show, area, query, &origin, &|name| low.contains(name));
    // Filtering shows what it found: every group that has a hit, open.
    let filtering = show != Show::All || area.is_some() || !query.trim().is_empty();
    let tally =
        |show| visible(show, area, "", &origin, &|_| false).iter().map(|(_, t)| t.len()).sum::<usize>();
    let (yours_count, base_count) = (tally(Show::Yours), tally(Show::Base));
    let low_count = low.iter().filter(|name| in_area(name, area)).count();

    let mut rows = Vec::new();
    if let Some(name) = inputs.selected.filter(|name| !in_area(name, area)) {
        rows.push(ListRow::NotInArea(wormhole_text::keys::studio_not_in_area(name, inputs.area_label)));
    }
    let mut joined = false;
    for (group, names) in shown {
        let open = filtering || inputs.open_groups.contains(group.name);
        rows.push(ListRow::Band(Band {
            name: group.name,
            open,
            toggles: !filtering,
            changed: names.iter().any(|name| changed(name)),
            low: names.iter().filter(|name| low.contains(**name)).count(),
            size: group_size(group, area),
            joined,
        }));
        joined = !open;
        if !open {
            continue;
        }
        rows.extend(names.into_iter().map(|name| {
            let (chip, value) = match tokens.metrics.get(name).filter(|_| is_metric(name)) {
                Some(v) => (Chip::Metric(v), format!("{}px", number(v))),
                None => {
                    let color = tokens.get(name).unwrap_or(MISSING_TOKEN);
                    (Chip::Color(color), color.to_hex())
                }
            };
            ListRow::Token(TokenRow {
                name: name.to_string(),
                chip,
                value: places::own_value(draft, inputs.shown, name)
                    .filter(|_| !is_metric(name))
                    .unwrap_or(value),
                selected: inputs.selected == Some(name),
                low: low.contains(name),
                changed: changed(name),
                origin: origin(name),
                reach: pair.then(|| places::reach(draft, inputs.shown, name)),
            })
        }));
    }
    Built { rows, yours_count, base_count, low_count }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use wormhole_core::theme::tokens::{Area, ThemeFile, ThemeTokens, builtin_tokens, resolve};

    use super::super::document::places::Origin;
    use super::super::document::set_override;
    use super::super::draft::{Show, draft_for, groups};
    use super::{Built, Inputs, ListRow, build};

    fn base() -> ThemeTokens {
        builtin_tokens().into_iter().next().expect("a built-in theme")
    }

    fn resolve_builtins(file: &ThemeFile) -> Result<ThemeTokens, String> {
        resolve(file, &|_| None)
    }

    fn rows(
        show: Show,
        area: Option<Area>,
        query: &str,
        open: &[&'static str],
        selected: Option<&str>,
    ) -> Built {
        let tokens = base();
        let initial = draft_for(&tokens, resolve_builtins);
        let mut draft = initial.clone();
        set_override(&mut draft, "primary", Some("#ff0000".into())).expect("sets primary");
        let low = BTreeSet::from(["primary".to_string()]);
        let open_groups: BTreeSet<&'static str> = open.iter().copied().collect();
        build(&Inputs {
            draft: &draft,
            initial: &initial,
            shown: None,
            tokens: &tokens,
            low: &low,
            show,
            area,
            area_label: "Sidebar",
            query,
            open_groups: &open_groups,
            selected,
        })
    }

    fn names(built: &Built) -> Vec<&str> {
        built
            .rows
            .iter()
            .map(|row| match row {
                ListRow::NotInArea(_) => "note",
                ListRow::Band(b) => b.name,
                ListRow::Token(t) => t.name.as_str(),
            })
            .collect()
    }

    #[test]
    fn folded_bands_are_one_row_each() {
        let built = rows(Show::All, None, "", &[], None);
        assert_eq!(built.rows.len(), groups().len());
        assert!(built.rows.iter().all(|row| matches!(row, ListRow::Band(b) if !b.open && b.toggles)));
    }

    #[test]
    fn an_open_band_lists_its_tokens_and_marks_what_changed() {
        let built = rows(Show::All, None, "", &["Primary"], Some("primary"));
        let at = names(&built).iter().position(|n| *n == "primary").expect("primary is listed");
        let ListRow::Token(token) = &built.rows[at] else { panic!("a token row") };
        assert!(token.selected && token.changed && token.low);
        assert_eq!(token.value, "#ff0000", "the draft's own value, as typed");
        let ListRow::Band(band) = &built.rows[at - 1] else { panic!("its band") };
        assert!(band.open && band.changed && band.low == 1);
        assert_eq!((built.yours_count, built.low_count), (1, 1));
        assert_eq!(token.origin, Origin::Yours);
    }

    #[test]
    fn a_filter_opens_every_band_that_has_a_hit_and_stops_folding() {
        let built = rows(Show::All, None, "primary", &[], None);
        let bands: Vec<_> =
            built.rows.iter().filter_map(|r| if let ListRow::Band(b) = r { Some(b) } else { None }).collect();
        assert!(!bands.is_empty() && bands.iter().all(|b| b.open && !b.toggles));
    }

    #[test]
    fn the_selected_token_the_area_leaves_out_gets_a_note() {
        let built = rows(Show::All, Some(Area::Sidebar), "", &[], Some("terminal.ansi.4"));
        assert!(
            matches!(&built.rows[0], ListRow::NotInArea(text) if text.starts_with("terminal.ansi.4 isn"))
        );
    }

    #[test]
    fn two_folded_bands_in_a_row_share_a_hairline() {
        let built = rows(Show::All, None, "", &[], None);
        let joined: Vec<bool> = built
            .rows
            .iter()
            .filter_map(|r| if let ListRow::Band(b) = r { Some(b.joined) } else { None })
            .collect();
        assert!(!joined[0] && joined[1..].iter().all(|j| *j));
    }

    #[test]
    fn the_yours_and_base_filters_list_what_the_theme_sets_and_what_it_still_has_as_copied() {
        let yours = rows(Show::Yours, None, "", &[], None);
        let listed = names(&yours);
        assert!(listed.contains(&"primary"), "{listed:?}");
        assert!(listed.iter().all(|n| matches!(*n, "Primary" | "primary")), "{listed:?}");
        // A built-in's draft sets its whole palette itself; with no base, nothing is "base".
        assert!(rows(Show::Base, None, "", &[], None).rows.is_empty());
    }

    #[test]
    fn a_line_is_the_same_line_when_only_its_contents_change() {
        let a = rows(Show::All, None, "", &["Primary"], None);
        let b = rows(Show::All, None, "", &["Primary"], Some("primary"));
        assert_eq!(a.rows.len(), b.rows.len());
        assert!(a.rows.iter().zip(&b.rows).all(|(x, y)| x.same_line(y)));
        assert_ne!(a.rows, b.rows, "the selection is a content change");
    }
}

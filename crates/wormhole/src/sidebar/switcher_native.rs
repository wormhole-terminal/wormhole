//! The tab-group switcher's data for the shared native picker.
use std::sync::RwLock;

use wormhole_core::view::picker::{Answer, Cancel, Indicator, IndicatorTone, Provider, Query, Row};
use wormhole_core::view::sidebar::Urgency;
use wormhole_core::view::sidebar::tabs::{Hit, TabSet, search};

use super::switcher::SwitchTarget;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Group {
    pub id: Option<u32>,
    pub label: String,
    pub members: Vec<usize>,
    pub current: bool,
    pub alerts: usize,
    pub color: Option<[u8; 3]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Terminal {
    pub id: i64,
    pub name: String,
    pub urgency: Urgency,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Catalog {
    pub groups: Vec<Group>,
    pub terminals: Vec<Terminal>,
    pub show_status: bool,
}

pub(super) struct Source(RwLock<Catalog>);

impl Source {
    pub fn new(catalog: Catalog) -> Self {
        Self(RwLock::new(catalog))
    }
    pub fn replace(&self, catalog: Catalog) {
        *self.0.write().unwrap_or_else(|e| e.into_inner()) = catalog;
    }
}

pub(super) enum Action {
    Open(SwitchTarget),
    NewGroup,
    ShowAll,
}

pub(super) fn key(target: SwitchTarget) -> String {
    match target {
        SwitchTarget::Group(Some(id)) => format!("g{id}"),
        SwitchTarget::Group(None) => "u".into(),
        SwitchTarget::Terminal(id) => format!("t{id}"),
    }
}

pub(super) fn action(key: &str) -> Option<Action> {
    if key == "u" {
        return Some(Action::Open(SwitchTarget::Group(None)));
    }
    if key == "new" {
        return Some(Action::NewGroup);
    }
    if key == "all" {
        return Some(Action::ShowAll);
    }
    if let Some(id) = key.strip_prefix('g').and_then(|id| id.parse().ok()) {
        return Some(Action::Open(SwitchTarget::Group(Some(id))));
    }
    key.strip_prefix('t').and_then(|id| id.parse().ok()).map(|id| Action::Open(SwitchTarget::Terminal(id)))
}

fn indicator(terminal: &Terminal) -> Indicator {
    let (symbol, status, tone) = match terminal.urgency {
        Urgency::NeedsYou => ("exclamationmark.circle.fill", "Needs input", IndicatorTone::Attention),
        Urgency::Failed => ("xmark.circle.fill", "Failed", IndicatorTone::Error),
        Urgency::Working => ("ellipsis.circle.fill", "Working", IndicatorTone::Active),
        Urgency::Bell => ("bell.fill", "Bell", IndicatorTone::Attention),
        Urgency::NewOutput => ("circle.fill", "New output", IndicatorTone::Active),
        Urgency::Running => ("play.circle.fill", "Running", IndicatorTone::Neutral),
        Urgency::Done => ("checkmark.circle.fill", "Done", IndicatorTone::Success),
        Urgency::Idle => ("circle", "Idle", IndicatorTone::Neutral),
    };
    Indicator { symbol: symbol.into(), label: format!("{}: {status}", terminal.name), tone }
}

impl Provider for Source {
    fn query(&self, query: &Query, cancel: &Cancel) -> Answer {
        let catalog = self.0.read().unwrap_or_else(|e| e.into_inner()).clone();
        let sets: Vec<_> = catalog
            .groups
            .iter()
            .map(|group| TabSet {
                band: group.id.map(|_| 0),
                label: group.label.clone(),
                members: group.members.clone(),
            })
            .collect();
        let group_row = |at: usize| {
            let group = &catalog.groups[at];
            let key = key(SwitchTarget::Group(group.id));
            let mut row = Row::new(&key, &group.label);
            row.icon = "folder".into();
            row.icon_color = group.color;
            let mut members: Vec<_> =
                group.members.iter().filter_map(|&ix| catalog.terminals.get(ix)).collect();
            members.sort_by_key(|terminal| terminal.urgency);
            if catalog.show_status {
                row.indicators = members.into_iter().map(indicator).collect();
            }
            let mut detail =
                wormhole_core::view::sidebar::wording::count(group.members.len(), "terminal", "terminals");
            if group.alerts > 0 {
                let verb = if group.alerts == 1 { "needs" } else { "need" };
                detail.push_str(&format!(" · {} {verb} attention", group.alerts));
            }
            let overflow = row.indicators.len().saturating_sub(crate::platform::picker::MAX_INDICATORS);
            if overflow > 0 {
                detail.push_str(&format!(" · +{overflow}"));
            }
            row.detail = Some(detail);
            row.badge = group.current.then(|| "✓".into());
            row.context_menu = group.id.is_some();
            row.insert = Some(key);
            row
        };
        let mut answer = Answer::default();
        if query.text.trim().is_empty() {
            answer.rows.extend((0..catalog.groups.len()).map(group_row));
            for (key, label, icon) in
                [("new", "New group", "plus"), ("all", "Show all groups in the sidebar", "sidebar.left")]
            {
                let mut row = Row::new(key, label);
                row.icon = icon.into();
                row.insert = Some(key.into());
                answer.rows.push(row);
            }
        } else {
            for hit in search(&sets, &query.text, |ix| {
                catalog.terminals.get(ix).map_or_else(String::new, |terminal| terminal.name.clone())
            }) {
                if cancel.is_cancelled() {
                    return Answer::default();
                }
                match hit {
                    Hit::Set(at) => answer.rows.push(group_row(at)),
                    Hit::Terminal(at, ix) => {
                        if let Some(terminal) = catalog.terminals.get(ix) {
                            let key = key(SwitchTarget::Terminal(terminal.id));
                            let mut row = Row::new(&key, &terminal.name);
                            row.icon = "terminal".into();
                            if catalog.show_status {
                                row.indicators.push(indicator(terminal));
                            }
                            row.detail = Some(catalog.groups[at].label.clone());
                            row.insert = Some(key);
                            answer.rows.push(row);
                        }
                    }
                }
            }
        }
        if cancel.is_cancelled() { Answer::default() } else { answer }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Catalog {
        Catalog {
            show_status: true,
            groups: vec![
                Group {
                    id: Some(7),
                    label: "Build".into(),
                    members: vec![0],
                    current: true,
                    alerts: 0,
                    color: Some([24, 120, 240]),
                },
                Group {
                    id: Some(9),
                    label: "Empty".into(),
                    members: vec![],
                    current: false,
                    alerts: 0,
                    color: None,
                },
                Group {
                    id: None,
                    label: "Ungrouped".into(),
                    members: vec![1],
                    current: false,
                    alerts: 1,
                    color: None,
                },
            ],
            terminals: vec![
                Terminal { id: 10, name: "server".into(), urgency: Urgency::Working },
                Terminal { id: 20, name: "build log".into(), urgency: Urgency::NeedsYou },
            ],
        }
    }

    #[test]
    fn groups_and_search_results_have_stable_actions_and_context_menus() {
        let source = Source::new(catalog());
        let ask =
            |text: &str| source.query(&Query { text: text.into(), ..Default::default() }, &Cancel::new());
        let rows = ask("").rows;
        assert_eq!(
            rows.iter().map(|row| row.key.as_str()).collect::<Vec<_>>(),
            ["g7", "g9", "u", "new", "all"]
        );
        assert!(rows[0].context_menu && rows[1].context_menu);
        assert!(!rows[2].context_menu);
        assert_eq!(rows[0].badge.as_deref(), Some("✓"));
        assert_eq!(rows[0].icon_color, Some([24, 120, 240]));
        assert_eq!(rows[0].indicators[0].label, "server: Working");
        assert_eq!(rows[2].indicators[0].label, "build log: Needs input");
        assert!(rows[2].detail.as_deref().unwrap().contains("1 needs attention"));
        assert_eq!(rows[1].detail.as_deref(), Some("0 terminals"));
        let matches = ask("build").rows;
        assert_eq!(matches.iter().map(|row| row.key.as_str()).collect::<Vec<_>>(), ["g7", "t20"]);
        assert!(matches!(action("g7"), Some(Action::Open(SwitchTarget::Group(Some(7))))));
        assert!(matches!(action("t20"), Some(Action::Open(SwitchTarget::Terminal(20)))));
        assert!(action("gX").is_none());
        let mut reordered = catalog();
        reordered.groups.swap(0, 1);
        source.replace(reordered);
        assert_eq!(ask("").rows[1].key, "g7");
    }

    #[test]
    fn status_cues_are_sorted_by_urgency_and_the_overflow_is_announced() {
        let mut data = catalog();
        data.terminals = vec![
            Terminal { id: 1, name: "idle".into(), urgency: Urgency::Idle },
            Terminal { id: 2, name: "failed".into(), urgency: Urgency::Failed },
            Terminal { id: 3, name: "working".into(), urgency: Urgency::Working },
            Terminal { id: 4, name: "finished".into(), urgency: Urgency::Done },
            Terminal { id: 5, name: "waiting".into(), urgency: Urgency::NeedsYou },
        ];
        data.groups.truncate(1);
        data.groups[0].members = (0..5).collect();
        data.groups[0].alerts = 2;
        let rows = Source::new(data).query(&Query::default(), &Cancel::new()).rows;
        let group = &rows[0];
        assert_eq!(
            group.indicators.iter().map(|indicator| indicator.label.as_str()).collect::<Vec<_>>(),
            ["waiting: Needs input", "failed: Failed", "working: Working", "finished: Done", "idle: Idle"]
        );
        assert!(group.detail.as_deref().unwrap().contains("2 need attention"));
        assert!(group.detail.as_deref().unwrap().contains("+1"));
    }

    #[test]
    fn disabling_status_symbols_hides_them_in_groups_and_search_results() {
        let mut data = catalog();
        data.show_status = false;
        let source = Source::new(data);
        let ask =
            |text: &str| source.query(&Query { text: text.into(), ..Default::default() }, &Cancel::new());
        let groups = ask("");
        assert!(groups.rows.iter().all(|row| row.indicators.is_empty()));
        assert!(groups.rows[2].detail.as_deref().unwrap().contains("1 needs attention"));
        assert!(ask("server").rows.iter().all(|row| row.indicators.is_empty()));
    }
}

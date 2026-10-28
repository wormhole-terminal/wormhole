//! The generic model driven end to end by a provider that knows nothing of folders: a fixed list of fruit.
//! That it works here is the proof that the component depends on nothing but its own data.

use std::sync::Arc;

use super::{
    Answer, Cancel, Compose, Context, Crumb, Decoration, Decorator, Field, Key, Model, Outcome, Provider,
    Query, Row, fit,
};

/// Fruit, each with a "colour" detail: typing fuzzy-matches the names; a name that ends in `s` can be gone into
/// (to list its varieties, here just itself with a slash).
struct Fruit(&'static [&'static str]);

impl Provider for Fruit {
    fn query(&self, query: &Query, _: &Cancel) -> Answer {
        let mut rows: Vec<Row> = self
            .0
            .iter()
            .filter_map(|name| {
                let found = fit(name, &query.text)?;
                Some(Row {
                    marks: found.marks,
                    score: found.score,
                    icon: "leaf".into(),
                    detail: Some("fruit".into()),
                    descend: Some(format!("{name}/")),
                    insert: Some(format!("eat {name}")),
                    forgettable: *name == "plum",
                    ..Row::new(*name, *name)
                })
            })
            .collect();
        rows.sort_by_key(|row| std::cmp::Reverse(row.score));
        Answer {
            rows,
            up: (!query.text.is_empty()).then(String::new),
            crumbs: vec![Crumb { label: "Fruit".into(), text: String::new() }],
            ..Answer::default()
        }
    }
}

struct Sweet;

impl Decorator for Sweet {
    fn decorate(&self, keys: &[String], _: &Cancel) -> Vec<Decoration> {
        keys.iter()
            .map(|key| Decoration { key: key.clone(), badge: Some("sweet".into()), chevron: Some(true) })
            .collect()
    }
}

const FRUIT: &[&str] = &["apple", "banana", "plum", "pineapple"];

/// What the driver does: ask, run the provider (here, at once) and hand the answer back.
fn settle(model: &mut Model, provider: &dyn Provider) -> Option<Outcome> {
    let request = model.request()?;
    let answer = provider.query(&request.query, &Cancel::new());
    model.deliver(request.generation, answer)
}

fn labels(model: &Model) -> Vec<String> {
    model
        .snapshot()
        .items
        .iter()
        .filter_map(|i| match i {
            super::Item::Row(r) => Some(r.label.clone()),
            _ => None,
        })
        .collect()
}

fn model() -> Model {
    Model::new(Context::default(), 9)
}

#[test]
fn a_provider_that_knows_no_folders_lists_selects_completes_and_picks() {
    let (mut model, fruit) = (model(), Fruit(FRUIT));
    settle(&mut model, &fruit);
    assert_eq!(labels(&model), ["apple", "banana", "plum", "pineapple"]);
    assert_eq!(model.snapshot().selected, Some(0));

    model.typed("pl");
    assert!(model.snapshot().busy, "the old rows stay while the answer is on its way");
    settle(&mut model, &fruit);
    assert_eq!(labels(&model), ["plum", "apple", "pineapple"], "prefix first, the rest by their fit");
    assert_eq!(model.snapshot().ghost.as_deref(), Some("um/"), "the ghost completes the selected row");
    let row_marks = match &model.snapshot().items[0] {
        super::Item::Row(r) => r.marks.clone(),
        _ => panic!("a row"),
    };
    assert_eq!(row_marks.len(), 1);
    assert_eq!(row_marks[0], 0..2);

    // → at the end of the text accepts the ghost: the field's text becomes the row's.
    let at_end = Field::at_end("pl");
    assert_eq!(model.key(Key::Right, &at_end), Outcome::SetText("plum/".into()));
    assert_eq!(model.snapshot().epoch, 1, "the panel is told to replace the field's text");
    assert_eq!(model.text(), "plum/");
}

#[test]
fn return_hands_over_the_rows_insert_text_and_a_click_picks_that_row() {
    let (mut model, fruit) = (model(), Fruit(FRUIT));
    settle(&mut model, &fruit);
    model.key(Key::Down, &Field::default());
    assert_eq!(model.key(Key::Enter, &Field::default()), Outcome::Insert("eat banana".into()));
    assert_eq!(model.click(0), Outcome::Insert("eat apple".into()));
    assert_eq!(model.click(99), Outcome::Nothing);
}

#[test]
fn an_answer_for_text_that_has_moved_on_is_dropped() {
    let (mut model, fruit) = (model(), Fruit(FRUIT));
    model.typed("a");
    let first = model.request().expect("asked");
    model.typed("ap");
    let second = model.request().expect("asked again");
    assert_ne!(first.generation, second.generation);
    let late = fruit.query(&first.query, &Cancel::new());
    assert_eq!(model.deliver(first.generation, late), None);
    assert!(!model.settled(), "still waiting for the answer to what is typed now");
    model.deliver(second.generation, fruit.query(&second.query, &Cancel::new()));
    assert!(model.settled());
    assert!(labels(&model).iter().all(|l| l.contains('a') || l.contains('p')));
}

#[test]
fn a_return_typed_before_the_answer_waits_for_it() {
    let (mut model, fruit) = (model(), Fruit(FRUIT));
    settle(&mut model, &fruit);
    model.typed("ban");
    assert_eq!(
        model.key(Key::Enter, &Field::at_end("ban")),
        Outcome::Nothing,
        "the list is for the old text"
    );
    let done = settle(&mut model, &fruit);
    assert_eq!(done, Some(Outcome::Insert("eat banana".into())));
}

#[test]
fn left_goes_up_by_the_providers_answer_and_a_crumb_goes_where_it_says() {
    let (mut model, fruit) = (model(), Fruit(FRUIT));
    model.typed("ban");
    settle(&mut model, &fruit);
    assert_eq!(
        model.key(Key::Left, &Field { text: "ban".into(), caret: 0, selected: 0 }),
        Outcome::SetText(String::new())
    );
    settle(&mut model, &fruit);
    assert_eq!(model.key(Key::Left, &Field::default()), Outcome::Pass, "the provider says nothing is above");
    model.typed("pl");
    settle(&mut model, &fruit);
    assert_eq!(model.crumb(0), Outcome::SetText(String::new()));
    assert_eq!(model.crumb(5), Outcome::Nothing);
}

#[test]
fn a_row_can_be_forgotten_only_when_it_says_so() {
    let (mut model, fruit) = (model(), Fruit(FRUIT));
    settle(&mut model, &fruit);
    assert_eq!(model.key(Key::Forget, &Field::default()), Outcome::Pass, "apple can't be forgotten");
    model.step(2);
    assert_eq!(model.key(Key::Forget, &Field::default()), Outcome::Forget("plum".into()));
    model.refresh();
    assert_eq!(model.snapshot().selected, Some(2), "the selection stays through a refresh");
}

#[test]
fn decorations_are_asked_for_the_rows_in_view_once_and_kept_for_when_they_come_back() {
    let (mut model, fruit) = (Model::new(Context::default(), 2), Fruit(FRUIT));
    settle(&mut model, &fruit);
    let first = model.decorations_wanted().expect("rows are in view");
    assert_eq!(first.keys, ["apple", "banana"], "only what is in view");
    assert!(model.decorations_wanted().is_none(), "and only once");
    model.decorated(Sweet.decorate(&first.keys, &Cancel::new()));
    let badges: Vec<_> = model
        .snapshot()
        .items
        .iter()
        .filter_map(|i| match i {
            super::Item::Row(r) => Some((r.badge.clone(), r.chevron)),
            _ => None,
        })
        .collect();
    assert_eq!(badges[0], (Some("sweet".into()), true));
    assert_eq!(badges[2], (None, false));
    model.scroll(2);
    let more = model.decorations_wanted().expect("the next rows came into view");
    assert_eq!(more.keys, ["plum", "pineapple"]);
    // The same rows listed again by a new answer carry what was found.
    model.typed("a");
    settle(&mut model, &fruit);
    assert!(
        model
            .snapshot()
            .items
            .iter()
            .any(|i| matches!(i, super::Item::Row(r) if r.key == "apple" && r.badge.is_some()))
    );
}

#[test]
fn the_selection_keeps_in_view_and_page_keys_stop_at_the_ends() {
    let many: &'static [&'static str] = &["a1", "a2", "a3", "a4", "a5", "a6", "a7"];
    let (mut model, list) = (Model::new(Context::default(), 3), Fruit(many));
    settle(&mut model, &list);
    for _ in 0..4 {
        model.key(Key::Down, &Field::default());
    }
    let s = model.snapshot();
    assert_eq!((s.selected, s.top, s.shown), (Some(4), 2, 3));
    model.key(Key::PageDown, &Field::default());
    assert_eq!(model.snapshot().selected, Some(6), "a page stops at the end");
    model.key(Key::Down, &Field::default());
    assert_eq!(model.snapshot().selected, Some(0), "an arrow wraps");
    assert_eq!(model.snapshot().top, 0);
}

#[test]
fn composed_providers_are_ranked_together_without_either_knowing_the_other() {
    let compose = Compose(vec![Arc::new(Fruit(&["apple", "plum"])), Arc::new(Fruit(&["pear", "apple"]))]);
    let answer = compose.query(&Query { text: "p".into(), ..Query::default() }, &Cancel::new());
    let keys: Vec<_> = answer.rows.iter().map(|r| r.key.as_str()).collect();
    assert_eq!(keys, ["plum", "pear", "apple"], "apple once, after the prefix fits");
}

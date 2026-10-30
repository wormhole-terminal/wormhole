//! The generic model driven end to end by a provider that knows nothing of folders: a fixed list of fruit.
//! That it works here is the proof that the component depends on nothing but its own data.

use std::sync::Arc;

use super::{
    Answer, Cancel, Compose, Context, Crumb, Decoration, Decorator, Field, Item, Key, Model, Note, Outcome,
    Provider, Query, Row, fit,
};

/// Fruit, each with a "colour" detail: typing fuzzy-matches the names; a name can be gone into (to list its
/// varieties, here just itself with a slash).
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
                    kind: "fruit".into(),
                    ..Row::new(*name, *name)
                })
            })
            .collect();
        rows.sort_by_key(|row| std::cmp::Reverse(row.score));
        Answer {
            rows,
            up: (!query.text.is_empty()).then(String::new),
            // Its text ends at a level when it ends with a dot: nothing to do with a slash.
            up_at_end: query.text.ends_with('.'),
            crumbs: vec![Crumb { label: "Fruit".into(), text: String::new() }],
            ..Answer::default()
        }
    }
}

struct Sweet;

impl Decorator for Sweet {
    fn kinds(&self) -> &[&'static str] {
        &["fruit"]
    }

    fn decorate(&self, keys: &[String], _: &Cancel) -> Vec<Decoration> {
        keys.iter()
            .map(|key| Decoration { key: key.clone(), badge: Some("sweet".into()), chevron: Some(true) })
            .collect()
    }
}

const FRUIT: &[&str] = &["apple", "banana", "plum", "pineapple"];

#[test]
fn font_preview_keeps_picker_rows_compatible_with_existing_providers() {
    let row = Row::new("apple", "Apple");
    let old = serde_json::to_value(&row).expect("row serializes");
    assert!(old.get("font_family").is_none());
    assert!(old.get("font_embedded").is_none());
    assert!(old.get("context_menu").is_none());
    assert!(old.get("icon_color").is_none());
    assert!(old.get("indicators").is_none());
    let restored: Row = serde_json::from_value(old).expect("an older row still reads");
    assert_eq!(restored.font_family, None);
    assert!(!restored.context_menu);
    assert!(restored.indicators.is_empty());
    let mut preview = row;
    preview.font_family = Some("Menlo".into());
    preview.context_menu = true;
    let encoded = serde_json::to_value(&preview).expect("preview serializes");
    assert_eq!(encoded.get("context_menu").and_then(|value| value.as_bool()), Some(true));
    let restored: Row = serde_json::from_value(encoded).expect("preview reads");
    assert_eq!(restored.font_family.as_deref(), Some("Menlo"));
}

/// What the driver does: ask, run the provider (here, at once) and hand the answer back.
fn settle(model: &mut Model, provider: &dyn Provider) -> Option<Outcome> {
    let request = model.request()?;
    let answer = provider.query(&request.query, &Cancel::new());
    model.deliver(request.generation, 0, answer)
}

fn labels(model: &Model) -> Vec<String> {
    model
        .items()
        .iter()
        .filter_map(|i| match i {
            Item::Row(r) => Some(r.label.clone()),
            _ => None,
        })
        .collect()
}

fn model() -> Model {
    Model::new(Context::default(), 9, 1)
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
        Item::Row(r) => r.marks.clone(),
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
fn refreshing_a_list_keeps_the_selected_row_when_its_position_changes() {
    let mut model = model();
    settle(&mut model, &Fruit(&["apple", "banana", "plum"]));
    model.key(Key::Down, &Field::default());
    assert_eq!(model.selected_row().map(|row| row.key.as_str()), Some("banana"));
    model.refresh();
    settle(&mut model, &Fruit(&["apricot", "apple", "banana", "plum"]));
    assert_eq!(model.selected_row().map(|row| row.key.as_str()), Some("banana"));
    assert_eq!(model.key(Key::Enter, &Field::default()), Outcome::Insert("eat banana".into()));
    model.typed("pl");
    settle(&mut model, &Fruit(&["apricot", "apple", "banana", "plum"]));
    assert_eq!(model.selected_row().map(|row| row.key.as_str()), Some("plum"));
}

#[test]
fn a_later_provider_can_improve_the_default_selection() {
    let mut model = Model::new(Context::default(), 9, 2);
    let request = model.request().expect("query");
    let mut lesser = Row::new("later", "Later");
    lesser.score = 1;
    let mut better = Row::new("better", "Better");
    better.score = 10;
    model.deliver(request.generation, 0, Answer { rows: vec![lesser], ..Answer::default() });
    model.deliver(request.generation, 1, Answer { rows: vec![better], ..Answer::default() });
    assert_eq!(model.selected_row().map(|row| row.key.as_str()), Some("better"));

    let mut model = Model::new(Context::default(), 9, 2);
    let request = model.request().expect("query");
    model.deliver(
        request.generation,
        0,
        Answer { rows: vec![Row::new("first", "First"), Row::new("chosen", "Chosen")], ..Answer::default() },
    );
    model.key(Key::Down, &Field::default());
    let mut better = Row::new("better", "Better");
    better.score = 10;
    model.deliver(request.generation, 1, Answer { rows: vec![better], ..Answer::default() });
    assert_eq!(model.selected_row().map(|row| row.key.as_str()), Some("chosen"));
}

#[test]
fn hovering_selects_a_row_without_scrolling_or_picking_a_header() {
    let mut model = Model::new(Context::default(), 2, 1);
    let request = model.request().expect("query");
    let rows = ["apple", "banana", "plum"]
        .into_iter()
        .map(|name| {
            let mut row = Row::new(name, name);
            row.insert = Some(name.into());
            row
        })
        .collect();
    model.deliver(
        request.generation,
        0,
        Answer { sections: vec!["Fruit".into()], rows, ..Answer::default() },
    );
    assert!(!model.hover_item(0, "apple"), "the section heading cannot be selected");
    model.scroll(2);
    let top = model.snapshot().top;
    assert_eq!(top, 2);
    model.set_window(2);
    assert_eq!(model.snapshot().top, top, "redrawing at the same size preserves scrolling");
    assert!(!model.hover_item(2, "apple"), "an event for an old row cannot select its replacement");
    assert_eq!(model.click_key(2, "apple"), Outcome::Nothing, "nor can a late click pick it");
    assert!(model.hover_item(2, "banana"));
    assert_eq!(model.snapshot().top, top, "hovering must not move the list");
    assert_eq!(model.selected_row().map(|row| row.label.as_str()), Some("banana"));
    assert_eq!(model.key(Key::Enter, &Field::default()), Outcome::Insert("banana".into()));
    assert!(!model.hover_item(99, "banana"));
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
    assert_eq!(model.deliver(first.generation, 0, late), None);
    assert!(!model.settled(), "still waiting for the answer to what is typed now");
    model.deliver(second.generation, 0, fruit.query(&second.query, &Cancel::new()));
    assert!(model.settled());
    assert!(labels(&model).iter().all(|l| l.contains('a') || l.contains('p')));
}

#[test]
fn text_the_person_typed_into_a_field_the_picker_has_since_replaced_is_dropped() {
    let (mut model, fruit) = (model(), Fruit(FRUIT));
    settle(&mut model, &fruit);
    model.typed("pl");
    settle(&mut model, &fruit);
    let before = model.epoch();
    // ⇥ replaces the text; a keystroke that was already on its way from the old field names the old epoch.
    assert_eq!(model.key(Key::Tab, &Field::at_end("pl")), Outcome::SetText("plum/".into()));
    model.typed_at(before, "plx");
    assert_eq!(model.text(), "plum/", "the stale edit is dropped");
    model.typed_at(model.epoch(), "plum/s");
    assert_eq!(model.text(), "plum/s", "an edit of the new field counts");
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
fn providers_answer_on_their_own_and_a_slow_one_holds_back_only_its_rows() {
    let mut model = Model::new(Context::default(), 9, 2);
    let request = model.request().expect("asked");
    let q = |fruit: &Fruit| fruit.query(&request.query, &Cancel::new());
    // The second provider answers first: its rows are listed, with no word about nothing being there.
    model.deliver(request.generation, 1, q(&Fruit(&["pear"])));
    assert_eq!(labels(&model), ["pear"]);
    assert!(!model.settled() && model.snapshot().busy);
    // A return now waits for the whole list…
    assert_eq!(model.key(Key::Enter, &Field::default()), Outcome::Nothing);
    // …until the deadline says the other is slow: the list says it is still reading and the key is done.
    assert_eq!(model.slow(request.generation), Some(Outcome::Insert("eat pear".into())));
    assert_eq!(model.items().last(), Some(&Item::Note(Note::Reading)));
    assert!(!model.snapshot().busy);
    // The late answer is still merged into the list, and the note goes.
    model.deliver(request.generation, 0, q(&Fruit(&["apple"])));
    assert_eq!(labels(&model), ["apple", "pear"]);
    assert!(model.settled());
    assert!(!model.items().iter().any(|i| matches!(i, Item::Note(_))));
    // An answer for an earlier question, or from no provider, is dropped.
    assert_eq!(model.deliver(request.generation + 7, 0, Answer::default()), None);
    assert_eq!(model.deliver(request.generation, 5, Answer::default()), None);
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
fn whether_left_at_the_end_goes_up_is_the_providers_to_say_not_the_pickers_to_guess() {
    let (mut model, fruit) = (model(), Fruit(FRUIT));
    model.typed("ban.");
    settle(&mut model, &fruit);
    // This provider's text ends at a level with a dot; a slash means nothing to it.
    assert!(model.facts().up_at_end);
    assert_eq!(model.key(Key::Left, &Field::at_end("ban.")), Outcome::SetText(String::new()));
    model.typed("ban/");
    settle(&mut model, &fruit);
    assert!(!model.facts().up_at_end);
    assert_eq!(model.key(Key::Left, &Field::at_end("ban/")), Outcome::Pass);
    // Text typed since the answer says nothing about where it ends.
    model.typed("ban.");
    settle(&mut model, &fruit);
    model.typed("bana.");
    assert!(!model.facts().up_at_end, "the answer was for the text before");
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
    let (mut model, fruit) = (Model::new(Context::default(), 2, 1), Fruit(FRUIT));
    settle(&mut model, &fruit);
    let first = model.decorations_wanted().expect("rows are in view");
    assert_eq!(first.keys(), ["apple", "banana"], "only what is in view");
    assert!(first.rows.iter().all(|t| t.kind == "fruit"), "with their kinds");
    assert!(model.decorations_wanted().is_none(), "and not again while they are being asked");
    let found = Sweet.decorate(&first.keys(), &Cancel::new());
    model.decorated(&first, found);
    let badges: Vec<_> = model
        .items()
        .iter()
        .filter_map(|i| match i {
            Item::Row(r) => Some((r.badge.clone(), r.chevron)),
            _ => None,
        })
        .collect();
    assert_eq!(badges[0], (Some("sweet".into()), true));
    assert_eq!(badges[2], (None, false));
    assert!(model.decorations_wanted().is_none(), "answered rows are not asked again");
    model.scroll(2);
    let more = model.decorations_wanted().expect("the next rows came into view");
    assert_eq!(more.keys(), ["plum", "pineapple"]);
    // The same rows listed again by a new answer carry what was found.
    model.typed("a");
    settle(&mut model, &fruit);
    assert!(model.items().iter().any(|i| matches!(i, Item::Row(r) if r.key == "apple" && r.badge.is_some())));
}

#[test]
fn rows_decorated_while_the_text_moves_on_still_get_what_was_found_and_a_lost_answer_is_given_up_on() {
    let (mut model, fruit) = (model(), Fruit(FRUIT));
    settle(&mut model, &fruit);
    let request = model.decorations_wanted().expect("rows are in view");
    // The person types on before the decorators answer: the text and the list change under the request.
    model.typed("a");
    settle(&mut model, &fruit);
    model.decorated(&request, Sweet.decorate(&request.keys(), &Cancel::new()));
    let sweet: Vec<_> = model
        .items()
        .iter()
        .filter_map(|i| match i {
            Item::Row(r) if r.badge.is_some() => Some(r.key.clone()),
            _ => None,
        })
        .collect();
    assert!(sweet.contains(&"apple".to_string()), "decorated although the text moved on: {sweet:?}");
    // A request whose answer never came: its rows are given up on, not asked about again (each ask is a thread).
    let mut model = Model::new(Context::default(), 9, 1);
    settle(&mut model, &fruit);
    let lost = model.decorations_wanted().expect("rows are in view");
    assert!(model.decorations_wanted().is_none());
    model.decorations_lost(&lost);
    assert!(model.decorations_wanted().is_none(), "none of them again");
    model.typed("a");
    settle(&mut model, &fruit);
    assert!(model.decorations_wanted().is_none(), "nor when they are listed again");
}

#[test]
fn the_decoration_budget_is_for_each_answer_not_for_the_pickers_life() {
    let names: Vec<&'static str> =
        (0..400).map(|n| -> &'static str { Box::leak(format!("f{n:03}").into_boxed_str()) }).collect();
    let many: &'static [&'static str] = Box::leak(names.into_boxed_slice());
    let (mut model, list) = (Model::new(Context::default(), 400, 1), Fruit(many));
    settle(&mut model, &list);
    let mut asked = 0;
    while let Some(request) = model.decorations_wanted() {
        asked += request.rows.len();
        let found = Sweet.decorate(&request.keys(), &Cancel::new());
        model.decorated(&request, found);
    }
    assert_eq!(asked, 300, "one answer is decorated up to its budget");
    // A new answer is a new budget; what was decorated is not asked again.
    model.typed("f3");
    settle(&mut model, &list);
    let next = model.decorations_wanted().expect("rows not decorated yet");
    assert!(
        next.rows.iter().all(|t| t.key.as_str() >= "f300"),
        "the ones already decorated are not asked again"
    );
}

#[test]
fn the_selection_keeps_in_view_and_page_keys_stop_at_the_ends() {
    let many: &'static [&'static str] = &["a1", "a2", "a3", "a4", "a5", "a6", "a7"];
    let (mut model, list) = (Model::new(Context::default(), 3, 1), Fruit(many));
    settle(&mut model, &list);
    for _ in 0..4 {
        model.key(Key::Down, &Field::default());
    }
    let s = model.snapshot();
    assert_eq!((s.selected, s.top, s.items.len(), s.lines), (Some(4), 2, 3, 7));
    model.key(Key::PageDown, &Field::default());
    assert_eq!(model.snapshot().selected, Some(6), "a page stops at the end");
    model.key(Key::Down, &Field::default());
    assert_eq!(model.snapshot().selected, Some(0), "an arrow wraps");
    assert_eq!(model.snapshot().top, 0);
}

#[test]
fn a_snapshot_holds_only_the_lines_in_view() {
    let many: &'static [&'static str] = &["a1", "a2", "a3", "a4", "a5", "a6", "a7", "a8"];
    let (mut model, list) = (Model::new(Context::default(), 3, 1), Fruit(many));
    settle(&mut model, &list);
    let s = model.snapshot();
    assert_eq!((s.items.len(), s.lines, s.top), (3, 8, 0));
    model.scroll(4);
    let s = model.snapshot();
    assert_eq!((s.top, s.items.len()), (4, 3));
    assert!(matches!(&s.items[0], Item::Row(r) if r.key == "a5"));
}

#[test]
fn composed_providers_are_ranked_together_without_either_knowing_the_other() {
    let compose = Compose(vec![Arc::new(Fruit(&["apple", "plum"])), Arc::new(Fruit(&["pear", "apple"]))]);
    let answer = compose.query(&Query { text: "p".into(), ..Query::default() }, &Cancel::new());
    let keys: Vec<_> = answer.rows.iter().map(|r| r.key.as_str()).collect();
    assert_eq!(keys, ["plum", "pear", "apple"], "apple once, after the prefix fits");
}

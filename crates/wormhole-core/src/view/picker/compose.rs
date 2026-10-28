//! Several providers as one: the rows of each, in sections, ranked together.

use std::sync::Arc;

use super::provider::{Cancel, Provider};
use super::types::{Answer, Query, Row};

/// Several providers as one provider, asked in turn. The picker's driver asks each provider on its own so a
/// slow one holds back only its rows (see `Model::deliver`); this is for a caller that wants one answer.
pub struct Compose(pub Vec<Arc<dyn Provider>>);

impl Provider for Compose {
    fn query(&self, query: &Query, cancel: &Cancel) -> Answer {
        let mut answers = Vec::new();
        for provider in &self.0 {
            if cancel.is_cancelled() {
                break;
            }
            answers.push(provider.query(query, cancel));
        }
        merge(answers)
    }
}

/// The answers merged: a section is one by its title (the untitled one is shared), so providers that answer
/// into the same section are ranked together by score, stably, and a title no earlier answer had is listed
/// after theirs. A row that two providers both list (the same key) is listed once: in the earlier section,
/// and within one section with the better score. `up` and the crumbs are the first answer's that has them.
pub fn merge(answers: Vec<Answer>) -> Answer {
    let mut merged = Answer { sections: Vec::new(), ..Answer::default() };
    for answer in answers {
        let section_of: Vec<usize> = answer
            .sections
            .iter()
            .map(|title| match merged.sections.iter().position(|t| t == title) {
                Some(at) => at,
                None => {
                    merged.sections.push(title.clone());
                    merged.sections.len() - 1
                }
            })
            .collect();
        for mut row in answer.rows {
            row.section = section_of.get(row.section).copied().unwrap_or(0);
            add(&mut merged.rows, row);
        }
        merged.notes.extend(answer.notes);
        if merged.up.is_none() && answer.up.is_some() {
            merged.up = answer.up;
            merged.up_at_end = answer.up_at_end;
        }
        if merged.crumbs.is_empty() {
            merged.crumbs = answer.crumbs;
        }
    }
    if merged.sections.is_empty() {
        merged.sections.push(String::new());
    }
    // Stable: rows with one score keep the order their provider gave.
    merged.rows.sort_by_key(|row| (row.section, std::cmp::Reverse(row.score)));
    merged
}

fn add(rows: &mut Vec<Row>, row: Row) {
    match rows.iter().position(|seen| seen.key == row.key) {
        None => rows.push(row),
        Some(at) if rows[at].section == row.section && row.score > rows[at].score => rows[at] = row,
        Some(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::merge;
    use crate::view::picker::types::{Answer, Row};

    fn answer(sections: &[&str], rows: &[(&str, usize, i32)]) -> Answer {
        Answer {
            sections: sections.iter().map(ToString::to_string).collect(),
            rows: rows
                .iter()
                .map(|&(key, section, score)| Row { section, score, ..Row::new(key, key) })
                .collect(),
            ..Answer::default()
        }
    }

    fn keys(a: &Answer) -> Vec<(&str, usize)> {
        a.rows.iter().map(|r| (r.key.as_str(), r.section)).collect()
    }

    #[test]
    fn the_untitled_section_is_shared_and_ranked_by_score() {
        let merged =
            merge(vec![answer(&[""], &[("a", 0, 10), ("b", 0, 30)]), answer(&[""], &[("c", 0, 20)])]);
        assert_eq!(keys(&merged), [("b", 0), ("c", 0), ("a", 0)]);
        assert_eq!(merged.sections, [""]);
    }

    #[test]
    fn equal_scores_keep_the_order_given() {
        let merged = merge(vec![answer(&[""], &[("z", 0, 0), ("a", 0, 0)]), answer(&[""], &[("m", 0, 0)])]);
        assert_eq!(keys(&merged), [("z", 0), ("a", 0), ("m", 0)]);
    }

    #[test]
    fn a_new_title_is_a_section_after_the_ones_before_it() {
        let merged = merge(vec![
            answer(&[""], &[("a", 0, 0)]),
            answer(&["Recent"], &[("r", 0, 5)]),
            answer(&["Recent"], &[("s", 0, 9)]),
        ]);
        assert_eq!(merged.sections, ["", "Recent"]);
        assert_eq!(keys(&merged), [("a", 0), ("s", 1), ("r", 1)]);
    }

    #[test]
    fn a_row_listed_twice_stays_in_the_earlier_section_or_with_the_better_score() {
        let merged = merge(vec![
            answer(&[""], &[("a", 0, 1), ("b", 0, 1)]),
            answer(&["Recent"], &[("a", 0, 99)]),
            answer(&[""], &[("b", 0, 7)]),
        ]);
        assert_eq!(keys(&merged), [("b", 0), ("a", 0)]);
        assert_eq!(merged.rows[0].score, 7);
    }

    #[test]
    fn nothing_at_all_still_has_a_section() {
        assert_eq!(merge(Vec::new()).sections, [""]);
    }
}

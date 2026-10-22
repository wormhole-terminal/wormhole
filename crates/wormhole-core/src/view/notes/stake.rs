//! What closing a terminal loses of its notes and tasks, in the words of the close question.

/// What closing says of `open_items` notes and tasks (the ones with something in them and the steps
/// not done): they are discarded. `None` when there are none, so the question says nothing. The
/// question puts its subject before it ("Its", for one terminal, or "Their").
pub fn discarded(open_items: usize) -> Option<&'static str> {
    (open_items > 0).then_some("notes and tasks are discarded.")
}

#[cfg(test)]
mod tests {
    use super::discarded;

    #[test]
    fn only_something_open_is_a_stake() {
        assert_eq!(discarded(0), None);
        assert_eq!(discarded(1), Some("notes and tasks are discarded."));
    }
}

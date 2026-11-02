//! Clear Buffer in a terminal whose session host may not be able to forget the history.

/// Whether Clear Buffer leaves the history in its host: only a hosted terminal whose host is too old to
/// forget it (`Some(false)`).
pub(super) fn history_kept_by_host(host_forgets: Option<bool>) -> bool {
    host_forgets == Some(false)
}

#[cfg(test)]
mod tests {
    use super::history_kept_by_host;

    #[test]
    fn clear_buffer_on_a_minor_1_host_says_so() {
        // A host of minor 1 skips ClearHistory: the history would come back, so the person is told.
        assert!(history_kept_by_host(Some(false)));
        // A current host forgets it, and a terminal in the app has nothing in a host: nothing to say.
        assert!(!history_kept_by_host(Some(true)));
        assert!(!history_kept_by_host(None));
    }
}

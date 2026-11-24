//! Which front end this process is: the GPUI app (the default) or the Swift app.
//!
//! Both draw their Settings from one table (`config::settings_index`), but a setting only one of them
//! paints must not show as a switch that does nothing in the other. The Swift app's bridge tells core
//! once, at start, that it is the Swift front end ([`set`]); every other process (the GPUI app, the
//! session host, the tools) is GPUI. After the first answer the fact never changes.
//!
//! Nothing but whether a settings row is shown (`Entry::with_needs`) may ask for now. A rule of what
//! the app does that depended on the front end would let the two apps decide differently where they
//! share this core to decide the same.

use std::sync::OnceLock;

/// A front end over this core.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontEnd {
    /// The GPUI app, and any process that never says otherwise.
    Gpui,
    /// The Swift app, through the FFI crate.
    Swift,
}

/// The process's answer: fixed by the first read or the first [`Fact::set`], whichever comes first, so
/// that two readers can never get different answers.
struct Fact(OnceLock<FrontEnd>);

impl Fact {
    const fn new() -> Self {
        Self(OnceLock::new())
    }

    fn get(&self) -> FrontEnd {
        *self.0.get_or_init(|| FrontEnd::Gpui)
    }

    fn set(&self, front_end: FrontEnd) -> bool {
        self.0.set(front_end).is_ok()
    }
}

static FACT: Fact = Fact::new();

/// Says which front end this process is. Call it once, before anything asks. Returns whether the answer
/// was taken: `false` when something had already asked (and so been told GPUI) or it was set before.
pub fn set(front_end: FrontEnd) -> bool {
    FACT.set(front_end)
}

/// The front end this process is, as [`set`] said or [`FrontEnd::Gpui`] if it never did.
pub fn current() -> FrontEnd {
    #[cfg(any(test, feature = "test-support"))]
    if let Some(front_end) = OVERRIDE.with(std::cell::Cell::get) {
        return front_end;
    }
    FACT.get()
}

/// Whether this is the Swift app.
pub fn is_swift() -> bool {
    current() == FrontEnd::Swift
}

#[cfg(any(test, feature = "test-support"))]
thread_local! {
    static OVERRIDE: std::cell::Cell<Option<FrontEnd>> = const { std::cell::Cell::new(None) };
}

/// Runs `body` as if this thread's process were `front_end`. For tests, which share a process: nobody
/// else sees the change, and the process's own answer is left alone.
#[cfg(any(test, feature = "test-support"))]
pub fn with_front_end<R>(front_end: FrontEnd, body: impl FnOnce() -> R) -> R {
    struct Restore(Option<FrontEnd>);
    impl Drop for Restore {
        fn drop(&mut self) {
            OVERRIDE.with(|cell| cell.set(self.0));
        }
    }
    let _restore = Restore(OVERRIDE.with(|cell| cell.replace(Some(front_end))));
    body()
}

#[cfg(test)]
mod tests {
    use super::{Fact, FrontEnd, current, is_swift, with_front_end};

    #[test]
    fn a_process_that_never_says_is_gpui() {
        assert_eq!(current(), FrontEnd::Gpui);
        assert!(!is_swift());
    }

    #[test]
    fn the_first_answer_stays() {
        let fact = Fact::new();
        assert!(fact.set(FrontEnd::Swift), "set before anyone asked");
        assert_eq!(fact.get(), FrontEnd::Swift);
        assert!(!fact.set(FrontEnd::Gpui), "a second answer is refused");
        assert_eq!(fact.get(), FrontEnd::Swift);
    }

    #[test]
    fn asking_first_fixes_the_default() {
        let fact = Fact::new();
        assert_eq!(fact.get(), FrontEnd::Gpui);
        assert!(!fact.set(FrontEnd::Swift), "it was already asked, and answered GPUI");
        assert_eq!(fact.get(), FrontEnd::Gpui);
    }

    #[test]
    fn a_test_can_be_the_swift_app_without_the_others_seeing_it() {
        with_front_end(FrontEnd::Swift, || {
            assert!(is_swift());
            with_front_end(FrontEnd::Gpui, || assert!(!is_swift(), "nested"));
            assert!(is_swift(), "restored after the inner one");
            assert!(
                !std::thread::spawn(is_swift).join().expect("thread"),
                "another thread is the process's own"
            );
        });
        assert!(!is_swift(), "restored after the body");
    }
}

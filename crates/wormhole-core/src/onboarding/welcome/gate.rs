//! Who may talk to the app as the welcome: the token rules (see the control protocol's `welcome`
//! module for why a token is enough). One [`Gate`] per welcome terminal, made when the app starts it
//! and shut when the welcome finishes.

use std::io::Read as _;

use crate::ipc::control::{Error, ErrorCode, WelcomeOp};

/// The variable the welcome terminal's environment carries its token in. The program removes it before
/// it hands the terminal to the shell.
pub const TOKEN_VAR: &str = "WORMHOLE_WELCOME_TOKEN";

/// A fresh random token: 32 hex digits (128 bits) from the system's random source, or `None` when it
/// can't be read, in which case the app starts no welcome (a token that can be guessed would be a
/// door left open).
pub fn new_token() -> Option<String> {
    let mut bytes = [0u8; 16];
    std::fs::File::open("/dev/urandom").ok()?.read_exact(&mut bytes).ok()?;
    Some(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// The door for one welcome terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gate {
    tab: i64,
    token: String,
    finished: bool,
}

impl Gate {
    pub fn new(tab: i64, token: String) -> Self {
        Self { tab, token, finished: false }
    }

    /// The terminal this gate belongs to.
    pub fn tab(&self) -> i64 {
        self.tab
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    /// The welcome ended: the token opens nothing any more.
    pub fn finish(&mut self) {
        self.finished = true;
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Whether a request `from` that tab with `token` may pass: it must come from this gate's terminal,
    /// carry its token, and arrive while the welcome runs. The reply never says which of the first two
    /// was wrong.
    pub fn check(&self, from: Option<i64>, token: &str) -> Result<(), Error> {
        let refuse = |message: &str| Err(Error::new(ErrorCode::Refused, message));
        if from != Some(self.tab) || !same(token, &self.token) {
            return refuse("this isn't the welcome's terminal, or its token");
        }
        if self.finished {
            return refuse("the welcome has finished");
        }
        Ok(())
    }
}

/// The gates of every welcome terminal the app has started. A request is answered only through
/// [`authorize`](Self::authorize), which is the one place that decides.
#[derive(Debug, Default)]
pub struct Registry {
    gates: Vec<Gate>,
}

impl Registry {
    /// A welcome starts in `tab`, with `token`. A tab has one gate: a second welcome in it replaces the
    /// first, whose token stops working.
    pub fn open(&mut self, tab: i64, token: String) {
        self.gates.retain(|gate| gate.tab != tab);
        self.gates.push(Gate::new(tab, token));
    }

    /// Whether `op`, sent `from` that tab with `token`, may be carried out: the terminal has a welcome,
    /// the token is its token, the welcome has not finished, and the op is within the caps. The error
    /// is what to answer.
    pub fn authorize(&self, from: Option<i64>, token: &str, op: &WelcomeOp) -> Result<(), Error> {
        let gate = from.and_then(|tab| self.gates.iter().find(|gate| gate.tab == tab));
        let Some(gate) = gate else {
            return Err(Error::new(ErrorCode::Refused, "this isn't the welcome's terminal, or its token"));
        };
        gate.check(from, token)?;
        op.check_caps()
    }

    /// The welcome in `tab` is over (or the terminal is gone): its token opens nothing.
    pub fn finish(&mut self, tab: i64) {
        if let Some(gate) = self.gates.iter_mut().find(|gate| gate.tab == tab) {
            gate.finish();
        }
    }

    /// Forget the terminal's gate (the terminal was closed).
    pub fn forget(&mut self, tab: i64) {
        self.gates.retain(|gate| gate.tab != tab);
    }

    /// Whether `tab` has a welcome that has not finished.
    pub fn running(&self, tab: i64) -> bool {
        self.gates.iter().any(|gate| gate.tab == tab && !gate.is_finished())
    }
}

/// `a == b` without stopping at the first difference, so how long the answer takes says nothing about
/// how much of a guess was right.
fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut difference = a.len() ^ b.len();
    for (x, y) in a.iter().zip(b) {
        difference |= usize::from(x ^ y);
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate() -> Gate {
        Gate::new(7, "0123456789abcdef0123456789abcdef".to_string())
    }

    #[test]
    fn the_right_tab_with_the_right_token_passes_while_the_welcome_runs() {
        assert_eq!(gate().check(Some(7), "0123456789abcdef0123456789abcdef"), Ok(()));
    }

    #[test]
    fn a_wrong_token_or_a_wrong_tab_is_refused() {
        let gate = gate();
        let refused = |from, token| gate.check(from, token).map_err(|e| e.code);
        assert_eq!(refused(Some(7), "0123456789abcdef0123456789abcdee"), Err(ErrorCode::Refused), "token");
        assert_eq!(refused(Some(7), ""), Err(ErrorCode::Refused), "no token");
        assert_eq!(refused(Some(7), "0123456789abcdef"), Err(ErrorCode::Refused), "a prefix of it");
        assert_eq!(refused(Some(8), "0123456789abcdef0123456789abcdef"), Err(ErrorCode::Refused), "tab");
        assert_eq!(refused(None, "0123456789abcdef0123456789abcdef"), Err(ErrorCode::Refused), "outside");
        // Both wrong say the same as one wrong: nothing to tell a guesser which part was right.
        let both = gate.check(Some(8), "nope").expect_err("refused");
        let token_only = gate.check(Some(7), "nope").expect_err("refused");
        assert_eq!(both, token_only);
    }

    #[test]
    fn after_the_welcome_finished_the_token_opens_nothing() {
        let mut gate = gate();
        gate.finish();
        assert!(gate.is_finished());
        let err = gate.check(Some(7), gate.token()).expect_err("shut");
        assert_eq!(err.code, ErrorCode::Refused);
        assert!(err.message.contains("finished"), "{}", err.message);
    }

    fn registry() -> Registry {
        let mut registry = Registry::default();
        registry.open(7, "seven".to_string());
        registry.open(9, "nine".to_string());
        registry
    }

    fn state() -> WelcomeOp {
        WelcomeOp::State
    }

    fn install() -> WelcomeOp {
        WelcomeOp::InstallHooks { agent: crate::agents::AgentId::Claude }
    }

    fn set_notify() -> WelcomeOp {
        WelcomeOp::SetNotify {
            needs_you: Some(true),
            agent_done: None,
            sound: Some(crate::config::NotifySound::All),
            allow: true,
        }
    }

    /// One of every op. The match fails to compile when an op is added without being listed here, so
    /// none gets past the door tests unchecked.
    fn every_op() -> Vec<WelcomeOp> {
        let ops = vec![
            state(),
            WelcomeOp::SetLook {
                theme: Some("Nord".into()),
                primary: None,
                ligatures: None,
                font_step: None,
            },
            set_notify(),
            WelcomeOp::SetShown {
                from: crate::config::presets::Shown::of(crate::config::presets::Preset::Balanced),
                shown: crate::config::presets::Shown::of(crate::config::presets::Preset::Zen),
            },
            install(),
            WelcomeOp::Demo { show: true },
            WelcomeOp::Import { source: "ghostty".into() },
            WelcomeOp::Finish { skipped: true },
        ];
        for op in &ops {
            match op {
                WelcomeOp::State
                | WelcomeOp::SetLook { .. }
                | WelcomeOp::SetNotify { .. }
                | WelcomeOp::SetShown { .. }
                | WelcomeOp::InstallHooks { .. }
                | WelcomeOp::Demo { .. }
                | WelcomeOp::Import { .. }
                | WelcomeOp::Finish { .. } => {}
            }
        }
        ops
    }

    /// Every op is behind the same door: the right tab with its own token, while its welcome runs.
    #[test]
    fn every_op_needs_its_terminals_token() {
        let ops = every_op();
        let registry = registry();
        for op in &ops {
            assert_eq!(registry.authorize(Some(7), "seven", op), Ok(()), "{op:?}");
            let refused = |from, token| registry.authorize(from, token, op).map_err(|e| e.code);
            assert_eq!(refused(Some(7), "nine"), Err(ErrorCode::Refused), "another terminal's token: {op:?}");
            assert_eq!(refused(Some(9), "seven"), Err(ErrorCode::Refused), "its token from another tab");
            assert_eq!(refused(Some(7), ""), Err(ErrorCode::Refused), "no token");
            assert_eq!(refused(Some(3), "seven"), Err(ErrorCode::Refused), "a terminal with no welcome");
            assert_eq!(refused(None, "seven"), Err(ErrorCode::Refused), "from outside any tab");
        }
    }

    #[test]
    fn once_it_has_finished_the_token_opens_nothing_and_others_are_unaffected() {
        let mut registry = registry();
        assert!(registry.running(7));
        registry.finish(7);
        assert!(!registry.running(7) && registry.running(9));
        for op in every_op() {
            let err = registry.authorize(Some(7), "seven", &op).expect_err("shut");
            assert_eq!(err.code, ErrorCode::Refused, "{op:?}");
        }
        assert_eq!(registry.authorize(Some(9), "nine", &install()), Ok(()));
    }

    #[test]
    fn a_closed_terminal_and_a_replaced_welcome_lose_their_tokens() {
        let mut registry = registry();
        registry.forget(9);
        assert!(registry.authorize(Some(9), "nine", &state()).is_err(), "the terminal is gone");
        registry.open(7, "again".to_string());
        assert!(registry.authorize(Some(7), "seven", &state()).is_err(), "the old token");
        assert!(registry.authorize(Some(7), "again", &state()).is_ok());
    }

    #[test]
    fn an_op_over_the_caps_is_refused_even_with_the_right_token() {
        let long = "x".repeat(crate::ipc::control::FIELD_MAX_BYTES + 1);
        let look = WelcomeOp::SetLook { theme: Some(long), primary: None, ligatures: None, font_step: None };
        let err = registry().authorize(Some(7), "seven", &look).expect_err("too long");
        assert_eq!(err.code, ErrorCode::TooLarge);
    }

    #[test]
    fn tokens_are_long_hex_and_differ() {
        let (a, b) = (new_token().expect("a token"), new_token().expect("a token"));
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    #[test]
    fn comparing_tokens_compares_all_of_them() {
        assert!(same("abc", "abc"));
        assert!(!same("abc", "abd"));
        assert!(!same("abc", "abcd"));
        assert!(!same("", "a"));
    }
}

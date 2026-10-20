//! The switches that let a program do more than draw: each one is off until the host turns it on.
//!
//! A terminal reads whatever a program (or a file it was told to `cat`, or the other end of an ssh
//! connection) sends, so anything that lets that text reach beyond the screen is a decision for
//! the host, not for the sequence. The default [`Policy`] is the locked-down one; a host opens
//! exactly what it has a use for, at the [`builder`](crate::Terminal::builder) or later with
//! [`Terminal::set_policy`](crate::Terminal::set_policy).

use crate::terminal::Terminal;

/// What a program may do besides print: every field is `false` in the default, and a new switch
/// added later will be `false` there too, so a host that spells out nothing never gets more than it
/// had.
///
/// The struct is `#[non_exhaustive]`: build one from [`Policy::default`] and set the fields.
///
/// ```
/// use wormhole_vt::Policy;
///
/// let mut policy = Policy::default();
/// policy.window_ops = true;
/// assert!(policy.window_ops && !policy.screen_checksum);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Policy {
    /// Whether a program may change the window (XTWINOPS: move, resize, iconify, raise, maximise,
    /// full screen). Off: the operations are ignored. On: each is an
    /// [`Event::WindowOp`](crate::Event::WindowOp) for the host to carry out, and a size in cells
    /// takes effect in the engine at once. Reading the title back is a switch of its own,
    /// [`title_report`](Self::title_report).
    pub window_ops: bool,
    /// Whether XTWINOPS 20 and 21, the reports of the icon name and the window title, are
    /// answered. A title is text the program (or a file `cat`ted into the terminal) chose, and the
    /// answer is typed into the input of whatever is running, usually the shell: cleaning the
    /// controls out does not stop it from becoming a command line the user then runs
    /// (CVE-2003-0063, CVE-2024-38395). Off: the requests are ignored like unknown sequences. Separate
    /// from [`window_ops`](Self::window_ops), which does not turn it on.
    pub title_report: bool,
    /// Whether DECRQCRA, the checksum of a rectangle of the screen, is answered. The checksum of a
    /// single cell tells a program what is in it, so any program, also one on the other end of an
    /// ssh connection, could read the screen back cell by cell. Off: the request is ignored like
    /// an unknown sequence and nothing is answered.
    pub screen_checksum: bool,
}

impl Terminal {
    /// What programs may do besides print, as the host last set it.
    pub fn policy(&self) -> Policy {
        self.policy
    }

    /// Change what programs may do. Takes effect for the next sequence; what an earlier one did
    /// stays done.
    pub fn set_policy(&mut self, policy: Policy) {
        self.policy = policy;
    }
}

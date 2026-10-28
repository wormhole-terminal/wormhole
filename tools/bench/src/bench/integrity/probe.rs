//! Is the terminal a fair witness? Asked once, before the first measured test.

use std::io::{self, Write};
use std::time::Duration;

use super::outcome::Why;
use super::plan::QUERY;
use crate::term::Input;

/// How many queries go out back to back. A terminal that answers one now and then, or only the
/// first of a burst, cannot vouch for anything.
pub const BURST: u64 = 16;
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Sends [`BURST`] queries and then DA1 (`ESC [ c`), which every terminal answers and which comes after
/// the others, so the answers to the queries are known to be all in, or never coming. `Ok(None)` when
/// the terminal can be trusted to count; otherwise why not.
///
/// The count goes through the same path the measured runs use, so what is proven here is what the
/// runs rely on. Leaves the counter at zero and counting on. `late` is how long answers that trail
/// the device attributes reply are waited for.
pub fn probe(input: &mut Input, out: &mut impl Write, late: Duration) -> io::Result<Option<Why>> {
    input.count_status_replies();
    input.take_status_replies();
    let burst: Vec<u8> = QUERY.repeat(BURST as usize);
    input.ask(out, &burst, PROBE_TIMEOUT)?;
    // The status reports never reach `ask` (they are counted as they are read), so what it saw of
    // them is in the count. DA1 comes after them on a terminal that answers in order; one that
    // answers it first, or is slow to answer at all, gets `late` to finish, DA1 or not.
    let got = input.wait_status_replies(BURST, late);
    input.take_status_replies();
    Ok(match got {
        BURST => None,
        0 => Some(Why::Silent),
        n if n > BURST => Some(Why::Surplus { answered: n, checkpoints: BURST }),
        n => Some(Why::Lossy { answered: n }),
    })
}

//! The seeds the checkpoints' places are drawn from.
//!
//! An invocation has one seed (`--verify-seed`, `check_seed` in the results); the seed of a plan is a
//! function of it and of *which* run the plan is for (the test, the run's number, the first try or the
//! repeat) and of nothing else. So a test's run lands on the same places whatever else ran before it,
//! and a repeat after a shortfall moves no other run's.

/// The largest seed: a JSON number holds integers up to 2^53 exactly, so every seed that is shown or
/// saved can be given back to `--verify-seed`.
pub const MAX_SEED: u64 = (1 << 53) - 1;

/// The run a plan is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot<'a> {
    pub test: &'a str,
    /// The run's number within the test, from 0.
    pub run: usize,
    /// 0 for the first try of the run, 1 for its patient repeat.
    pub attempt: usize,
}

/// SplitMix64's finaliser: every input bit moves every output bit, so neighbouring inputs (two clock
/// readings, two run numbers) give unrelated outputs. Fixed forever: seeds are written to files.
fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Into 1..=[`MAX_SEED`]: the generator takes a seed of 0 as 1, and keeping 0 out of the range keeps
/// what is reported what was used.
fn into_range(x: u64) -> u64 {
    (x & MAX_SEED).max(1)
}

/// A seed for an invocation from the clock (nanoseconds since the epoch, which are far above
/// [`MAX_SEED`]: they are mixed down, not clamped).
pub fn from_clock(nanos: u64) -> u64 {
    into_range(mix(nanos))
}

/// A seed given by the user, held to the range.
pub fn given(seed: u64) -> u64 {
    seed.clamp(1, MAX_SEED)
}

/// The seed of the plan for `slot` in an invocation seeded `base`.
pub fn for_slot(base: u64, slot: Slot<'_>) -> u64 {
    let mut h = mix(base);
    for b in slot.test.bytes() {
        h = mix(h ^ u64::from(b));
    }
    // The length and the numbers go in after the name, so that no name runs into them.
    h = mix(h ^ slot.test.len() as u64);
    h = mix(h ^ slot.run as u64);
    h = mix(h ^ slot.attempt as u64);
    into_range(h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_readings_give_different_seeds_inside_the_range() {
        // Nanoseconds since the epoch are about 1.8e18, thirty times past the range.
        let base = 1_790_000_000_000_000_000u64;
        let seeds: Vec<u64> = (0..1000).map(|i| from_clock(base + i * 1_000)).collect();
        assert!(seeds.iter().all(|s| (1..=MAX_SEED).contains(s)));
        let mut sorted = seeds.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), seeds.len());
        assert_ne!(from_clock(base), MAX_SEED);
        // Even the ends of the u64 range land inside the range.
        for n in [0, 1, u64::MAX] {
            assert!((1..=MAX_SEED).contains(&from_clock(n)), "{n}");
        }
    }

    #[test]
    fn a_given_seed_is_held_to_the_range() {
        assert_eq!([given(0), given(7), given(MAX_SEED), given(u64::MAX)], [1, 7, MAX_SEED, MAX_SEED]);
    }

    #[test]
    fn a_runs_seed_depends_on_the_invocation_and_the_run_and_on_nothing_else() {
        let slot = Slot { test: "logflood", run: 1, attempt: 0 };
        let seed = for_slot(42, slot);
        assert_eq!(seed, for_slot(42, slot));
        assert!((1..=MAX_SEED).contains(&seed));
        let others = [
            for_slot(43, slot),
            for_slot(42, Slot { test: "colorflood", ..slot }),
            for_slot(42, Slot { run: 2, ..slot }),
            for_slot(42, Slot { attempt: 1, ..slot }),
        ];
        for (i, other) in others.iter().enumerate() {
            assert_ne!(*other, seed, "{i}");
        }
        // Written down: seeds are in saved files, and a change of the derivation would break replays.
        assert_eq!(for_slot(42, slot), 2_003_616_868_172_255);
    }
}

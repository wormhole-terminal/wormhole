//! Where to cut a stream.

use crate::rng::Rng;

/// Every offset `0..=len`: for streams short enough to afford it (the `.vtt` cases).
pub fn every_byte(len: usize) -> Vec<usize> {
    (0..=len).collect()
}

/// About `count` offsets of `bytes`, chosen to land where a repaint is hardest to get right: half
/// of them just inside escape sequences (after an ESC, after the introducer, in the middle of a
/// string) and inside multi-byte characters, the rest spread evenly with some jitter; plus the
/// start, the end and the first bytes. The same `seed` gives the same offsets.
pub fn select(bytes: &[u8], count: usize, seed: u64) -> Vec<usize> {
    let n = bytes.len();
    let mut rng = Rng::new(seed ^ n as u64);
    let mut cuts = vec![0, n];
    // Offsets that cut something in two.
    let mut inside = Vec::new();
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            0x1b => {
                // After the ESC, and after its introducer.
                inside.push(i + 1);
                if i + 2 <= n && matches!(bytes.get(i + 1), Some(b'[' | b']' | b'P' | b'_' | b'(' | b')')) {
                    inside.push(i + 2);
                }
            }
            // A continuation byte: the cut falls inside the character before it.
            0x80..=0xbf => inside.push(i),
            // The separator inside OSC, DCS and APC strings and CSI parameters.
            b';' | b':' => inside.push(i + 1),
            _ => {}
        }
    }
    let half = count / 2;
    if !inside.is_empty() {
        for _ in 0..half {
            cuts.push(inside[rng.below(inside.len())].min(n));
        }
    }
    let rest = count - half;
    for k in 0..rest {
        let base = n * k / rest.max(1);
        let jitter = if n > rest { rng.below(n / rest.max(1) + 1) } else { 0 };
        cuts.push((base + jitter).min(n));
    }
    cuts.extend((1..=24).filter(|&i| i <= n));
    cuts.sort_unstable();
    cuts.dedup();
    cuts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuts_include_the_ends_and_the_insides_of_sequences() {
        let bytes = "ab\u{65e5}\x1b[31mcd\x1b]8;;http://x\x07ef".as_bytes();
        let cuts = select(bytes, 60, 1);
        assert!(cuts.contains(&0) && cuts.contains(&bytes.len()));
        assert!(cuts.windows(2).all(|w| w[0] < w[1]), "sorted and unique");
        // Between the ESC and its `[`, and inside the three-byte character.
        let esc = bytes.iter().position(|&b| b == 0x1b).expect("an ESC");
        assert!(cuts.contains(&(esc + 1)));
        assert!(cuts.contains(&3) || cuts.contains(&4));
        assert_eq!(select(bytes, 60, 1), cuts, "deterministic");
    }
}

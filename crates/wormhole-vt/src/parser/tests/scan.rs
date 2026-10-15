//! The run scanners against the obvious loops.

use super::property::Rng;
use crate::parser::scan::{first_control, printable_run};

fn naive_run(bytes: &[u8]) -> usize {
    bytes.iter().take_while(|b| (0x20..=0x7E).contains(*b)).count()
}

fn naive_control(bytes: &[u8]) -> usize {
    bytes.iter().take_while(|&&b| b >= 0x20).count()
}

#[test]
fn every_length_and_every_stopper_position() {
    // Lengths across the 16 and 64 byte strides, a stopper byte of each kind at every position.
    for len in 0..200 {
        let text = vec![b'a'; len];
        assert_eq!(printable_run(&text), len);
        assert_eq!(first_control(&text), len);
        for at in 0..len {
            for stop in [0x00u8, 0x1F, 0x20, 0x7E, 0x7F, 0x80, 0xFF] {
                let mut bytes = text.clone();
                bytes[at] = stop;
                assert_eq!(printable_run(&bytes), naive_run(&bytes), "{stop:#x} at {at} of {len}");
                assert_eq!(first_control(&bytes), naive_control(&bytes), "{stop:#x} at {at} of {len}");
            }
        }
    }
}

#[test]
fn random_bytes() {
    let mut rng = Rng::new(0x5CA7);
    for _ in 0..50_000 {
        let len = rng.below(150);
        // Mostly printable, so that the runs are long enough to matter.
        let bytes: Vec<u8> = (0..len)
            .map(|_| if rng.below(40) == 0 { rng.next_u64() as u8 } else { 0x20 + rng.below(0x5F) as u8 })
            .collect();
        assert_eq!(printable_run(&bytes), naive_run(&bytes), "{bytes:x?}");
        assert_eq!(first_control(&bytes), naive_control(&bytes), "{bytes:x?}");
    }
}

#[test]
fn every_byte_value_alone_and_after_text() {
    for b in 0..=255u8 {
        for lead in [0usize, 7, 8, 15, 16, 17, 63, 64, 65] {
            let mut bytes = vec![b'x'; lead];
            bytes.push(b);
            bytes.extend_from_slice(b"yyyy");
            assert_eq!(printable_run(&bytes), naive_run(&bytes), "{b:#x} after {lead}");
            assert_eq!(first_control(&bytes), naive_control(&bytes), "{b:#x} after {lead}");
        }
    }
}

//! Numbers behind the layer's constants, run by hand with `--ignored --nocapture`.

use std::time::{Duration, Instant};

use super::{SIZE, launch};
use crate::terminal::pty::{open, spawn};

/// What the kernel hands one `read` of the master, by how the program writes and how fast the
/// reader is. Run with `--ignored --nocapture`.
#[test]
#[ignore = "measurement; run with --ignored --nocapture"]
fn read_sizes() {
    use std::io::Read as _;
    use std::os::fd::AsRawFd as _;

    let big = "x".repeat(100);
    let cases: [(&str, String); 3] = [
        ("`yes` (one write per line block)", format!("yes '{big}' | head -c 67108864")),
        ("`dd` 1 MiB writes", "dd if=/dev/zero bs=1048576 count=64 2>/dev/null | tr '\\0' 'x'".into()),
        (
            "`printf` per line (tiny writes)",
            "i=0; while [ $i -lt 20000 ]; do printf 'line %s\\n' $i; i=$((i+1)); done".into(),
        ),
    ];
    for (name, script) in cases {
        for (pace, pause, vectored) in [
            ("as fast as it can", Duration::ZERO, false),
            ("100 µs of parsing per read", Duration::from_micros(100), false),
            ("as fast as it can, readv of 256 KiB", Duration::ZERO, true),
        ] {
            let open::Pair { master, slave } = open::open(SIZE).expect("opens");
            let mut child = spawn::spawn(&launch("/bin/sh", &["-c", &script]), slave).expect("starts");
            let mut buf = vec![0u8; 1 << 20];
            let (mut sizes, mut total) = (Vec::new(), 0usize);
            let started = Instant::now();
            loop {
                let mut poll = libc::pollfd { fd: master.as_raw_fd(), events: libc::POLLIN, revents: 0 };
                #[allow(unsafe_code)]
                // SAFETY: one valid pollfd.
                unsafe {
                    libc::poll(&mut poll, 1, 1000);
                }
                let read = if vectored {
                    let mut slices: Vec<std::io::IoSliceMut<'_>> =
                        buf.chunks_mut(1024).take(256).map(std::io::IoSliceMut::new).collect();
                    std::io::Read::read_vectored(&mut &master, &mut slices)
                } else {
                    (&master).read(&mut buf)
                };
                match read {
                    Ok(0) => break,
                    Ok(n) => {
                        sizes.push(n);
                        total += n;
                        if !pause.is_zero() {
                            let until = Instant::now() + pause;
                            while Instant::now() < until {
                                std::hint::spin_loop();
                            }
                        }
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(_) => break,
                }
            }
            let took = started.elapsed();
            let _ = child.wait();
            sizes.sort_unstable();
            let pct = |p: usize| sizes.get(sizes.len().saturating_sub(1) * p / 100).copied().unwrap_or(0);
            eprintln!(
                "{name}, reader {pace}: {} reads, {:.0} MB/s, read size median {} p90 {} p99 {} max {}",
                sizes.len(),
                total as f64 / took.as_secs_f64() / 1e6,
                pct(50),
                pct(90),
                pct(99),
                sizes.last().copied().unwrap_or(0),
            );
        }
    }
}

/// How much a program can write to a terminal nobody reads before its write would block (the
/// kernel's output queue, which is also all one `read` of the master returns). Run with
/// `--ignored --nocapture`.
#[test]
#[ignore = "measurement; run with --ignored --nocapture"]
#[allow(unsafe_code)]
fn output_queue() {
    use std::io::Write as _;
    use std::os::fd::AsRawFd as _;

    let open::Pair { master: _master, slave } = open::open(SIZE).expect("opens");
    let fd = slave.as_raw_fd();
    // SAFETY: a plain fcntl on a descriptor that is open.
    unsafe {
        libc::fcntl(fd, libc::F_SETFL, libc::fcntl(fd, libc::F_GETFL) | libc::O_NONBLOCK);
    }
    let mut file = std::fs::File::from(slave);
    let mut queued = 0;
    while file.write(b"x").is_ok() {
        queued += 1;
    }
    eprintln!("{queued} bytes queued before the write would block");
}

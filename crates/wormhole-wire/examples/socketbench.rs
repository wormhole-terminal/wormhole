//! The socket bench of `session-host.md` section 14, P0: what the wire codec costs on a Unix
//! socket between two threads (two processes would add only the scheduler).
//!
//! * latency: a one-byte `Input` to the "host", a one-byte `Out` back, 20,000 round trips, p50 and p99
//!   in microseconds, next to the same ping-pong with raw one-byte writes. Two hops each.
//! * throughput: `Out` frames of 64 KiB (a flood) and 1 KiB (what a macOS PTY read yields) streamed
//!   one way, decoded and order-checked on the other side, in MB/s, next to raw writes of the same size.
//!
//!     cargo run --release -p wormhole-wire --example socketbench [-- --samples 20000 --mb 2048]
//!
//! Options: `--socketpair` (an unnamed pair instead of a bound socket), `--buffer-kib N` (SO_SNDBUF and SO_RCVBUF).
//!
//! Section 4 measured the bare socket (C, socketpair): 2 hops p50 3.2 to 4.7 us, p99 19 to 23 us,
//! 64 KiB streaming 11.9 to 12.7 GB/s. The budget (section 13) is +50 us p50 over in-process.
#![allow(clippy::print_stdout)]
#![allow(unsafe_code)] // one setsockopt, below

use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

use wormhole_wire::io::{FrameReader, FrameWriter};
use wormhole_wire::{CHANNEL_LOCAL, Message, Out, StreamOrder};

static NEXT: AtomicUsize = AtomicUsize::new(0);
static SOCKETPAIR: AtomicBool = AtomicBool::new(false);
/// SO_SNDBUF and SO_RCVBUF in KiB; 0 leaves the system's default, which limits a flood on macOS.
static BUFFER_KIB: AtomicUsize = AtomicUsize::new(0);

fn set_buffers(s: &UnixStream, kib: usize) {
    let bytes = (kib * 1024) as libc::c_int;
    for opt in [libc::SO_SNDBUF, libc::SO_RCVBUF] {
        // SAFETY: the fd is open for the life of `s`, and the option's value is a c_int whose size is passed.
        let rc = unsafe {
            libc::setsockopt(
                s.as_raw_fd(),
                libc::SOL_SOCKET,
                opt,
                (&raw const bytes).cast(),
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            )
        };
        assert_eq!(rc, 0, "setsockopt: {}", std::io::Error::last_os_error());
    }
}

/// A connected pair, with the buffer size asked for.
fn pair() -> (UnixStream, UnixStream) {
    let (a, b) = unbuffered_pair();
    let kib = BUFFER_KIB.load(Ordering::Relaxed);
    if kib > 0 {
        set_buffers(&a, kib);
        set_buffers(&b, kib);
    }
    (a, b)
}

/// A connected pair over a real bound socket in a private temporary directory.
fn unbuffered_pair() -> (UnixStream, UnixStream) {
    if SOCKETPAIR.load(Ordering::Relaxed) {
        return UnixStream::pair().expect("socketpair");
    }
    let dir = std::env::temp_dir().join(format!(
        "wh-wire-bench-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("s.sock");
    let listener = UnixListener::bind(&path).expect("bind");
    let client = UnixStream::connect(&path).expect("connect");
    let (server, _) = listener.accept().expect("accept");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
    (client, server)
}

fn percentile(sorted: &[u64], p: f64) -> f64 {
    sorted[((sorted.len() - 1) as f64 * p) as usize] as f64 / 1000.0
}

fn report(name: &str, mut ns: Vec<u64>) {
    ns.sort_unstable();
    println!(
        "{name:<26} p50 {:>7.1} us   p90 {:>7.1} us   p99 {:>7.1} us   max {:>8.1} us",
        percentile(&ns, 0.50),
        percentile(&ns, 0.90),
        percentile(&ns, 0.99),
        *ns.last().unwrap_or(&0) as f64 / 1000.0
    );
}

fn raw_latency(samples: usize) -> Vec<u64> {
    let (mut a, mut b) = pair();
    let echo = std::thread::spawn(move || {
        let mut byte = [0u8; 1];
        while b.read_exact(&mut byte).is_ok() {
            if b.write_all(&byte).is_err() {
                break;
            }
        }
    });
    let mut out = Vec::with_capacity(samples);
    let mut byte = [0u8; 1];
    for i in 0..samples + 2000 {
        let t = Instant::now();
        a.write_all(&[7]).expect("write");
        a.read_exact(&mut byte).expect("read");
        if i >= 2000 {
            out.push(t.elapsed().as_nanos() as u64);
        }
    }
    drop(a);
    let _ = echo.join();
    out
}

fn framed_latency(samples: usize) -> Vec<u64> {
    let (a, b) = pair();
    let host = std::thread::spawn(move || {
        let mut r = FrameReader::new(b.try_clone().expect("clone"));
        let mut w = FrameWriter::new(b);
        let mut offset = 0u64;
        while let Ok(Some(frame)) = r.read_frame() {
            if let Message::Input(bytes) = frame.message {
                let out = Message::Out(Out { offset, bytes });
                offset += bytes.len() as u64;
                if w.send(frame.channel, 0, &out).is_err() {
                    break;
                }
            }
        }
    });
    let mut r = FrameReader::new(a.try_clone().expect("clone"));
    let mut w = FrameWriter::new(a);
    let mut order = StreamOrder::new();
    let mut out = Vec::with_capacity(samples);
    for i in 0..samples + 2000 {
        let t = Instant::now();
        w.send(CHANNEL_LOCAL, 0, &Message::Input(&[7])).expect("send");
        let frame = r.read_frame().expect("read").expect("an echo");
        if let Message::Out(o) = frame.message {
            order.out(&o).expect("in order");
        }
        if i >= 2000 {
            out.push(t.elapsed().as_nanos() as u64);
        }
    }
    drop(w);
    drop(r);
    let _ = host.join();
    out
}

/// MB/s of `total` bytes in blocks of `block`, written raw and read raw.
fn raw_throughput(total: usize, block: usize) -> f64 {
    let (mut a, mut b) = pair();
    let reader = std::thread::spawn(move || {
        let mut buf = vec![0u8; 1 << 16];
        let mut n = 0;
        while let Ok(k) = b.read(&mut buf) {
            if k == 0 {
                break;
            }
            n += k;
        }
        n
    });
    let data = vec![0x5Au8; block];
    let t = Instant::now();
    let mut sent = 0;
    while sent < total {
        a.write_all(&data).expect("write");
        sent += block;
    }
    drop(a);
    let got = reader.join().expect("reader");
    assert_eq!(got, sent);
    sent as f64 / t.elapsed().as_secs_f64() / 1e6
}

/// MB/s of payload bytes in `Out` frames of `block` bytes, decoded and order-checked.
fn framed_throughput(total: usize, block: usize) -> f64 {
    let (a, b) = pair();
    let reader = std::thread::spawn(move || {
        let mut r = FrameReader::new(b);
        let mut order = StreamOrder::new();
        let mut n = 0;
        while let Ok(Some(frame)) = r.read_frame() {
            if let Message::Out(o) = frame.message {
                order.out(&o).expect("contiguous");
                n += o.bytes.len();
            }
        }
        n
    });
    let data = vec![0x5Au8; block];
    let mut w = FrameWriter::new(a);
    let t = Instant::now();
    let (mut sent, mut offset) = (0, 0u64);
    while sent < total {
        w.send(CHANNEL_LOCAL, 0, &Message::Out(Out { offset, bytes: &data })).expect("send");
        sent += block;
        offset += block as u64;
    }
    drop(w);
    let got = reader.join().expect("reader");
    assert_eq!(got, sent);
    sent as f64 / t.elapsed().as_secs_f64() / 1e6
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |name: &str, default: usize| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    BUFFER_KIB.store(arg("--buffer-kib", 0), Ordering::Relaxed);
    SOCKETPAIR.store(args.iter().any(|a| a == "--socketpair"), Ordering::Relaxed);
    let (samples, mb) = (arg("--samples", 20_000), arg("--mb", 2048));
    println!("wormhole-wire socket bench: {samples} round trips, {mb} MB per throughput row\n");
    report("raw 1-byte ping-pong", raw_latency(samples));
    report("framed Input -> Out", framed_latency(samples));
    println!();
    let total = mb * 1_000_000;
    for block in [64 * 1024, 16 * 1024, 1024] {
        let raw = raw_throughput(total, block);
        let framed = framed_throughput(total, block);
        println!(
            "{:>6} B blocks: raw {:>8.0} MB/s   framed {:>8.0} MB/s   ({:.0}% of raw)",
            block,
            raw,
            framed,
            100.0 * framed / raw
        );
    }
}

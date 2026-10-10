//! What machine and day a result belongs to. Everything falls back to something readable: a result
//! file without a CPU name is still a result.

use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Machine {
    pub cpu: String,
    pub model: String,
    pub os: String,
    pub memory: String,
    pub cores: usize,
}

fn command(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

pub fn machine() -> Machine {
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0);
    let fallback = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
    let mut m = Machine { cores, ..Machine::default() };
    if cfg!(target_os = "macos") {
        m.cpu = command("sysctl", &["-n", "machdep.cpu.brand_string"]).unwrap_or_default();
        m.model = command("sysctl", &["-n", "hw.model"]).unwrap_or_default();
        m.os = command("sw_vers", &["-productVersion"]).map(|v| format!("macOS {v}")).unwrap_or_default();
        m.memory = command("sysctl", &["-n", "hw.memsize"])
            .and_then(|b| b.parse::<u64>().ok())
            .map(|b| format!("{} GB", (b as f64 / (1u64 << 30) as f64).round() as u64))
            .unwrap_or_default();
    } else {
        m.cpu =
            std::fs::read_to_string("/proc/cpuinfo").ok().and_then(|t| cpuinfo_model(&t)).unwrap_or_default();
        m.model = std::fs::read_to_string("/sys/class/dmi/id/product_name")
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        m.os = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|t| os_pretty_name(&t))
            .unwrap_or_default();
        m.memory =
            std::fs::read_to_string("/proc/meminfo").ok().and_then(|t| meminfo_gb(&t)).unwrap_or_default();
    }
    if m.cpu.is_empty() {
        m.cpu = fallback.clone();
    }
    if m.os.is_empty() {
        m.os = fallback;
    }
    m
}

/// The first `model name` line of /proc/cpuinfo.
fn cpuinfo_model(text: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.strip_prefix("model name").and_then(|r| r.split_once(':')).map(|(_, v)| v.trim()))
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

fn os_pretty_name(text: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .map(|v| v.trim().trim_matches('"').to_string())
        .filter(|v| !v.is_empty())
}

fn meminfo_gb(text: &str) -> Option<String> {
    let kb: u64 =
        text.lines().find_map(|l| l.strip_prefix("MemTotal:"))?.trim().split(' ').next()?.parse().ok()?;
    Some(format!("{} GB", (kb as f64 / (1024.0 * 1024.0)).round() as u64))
}

/// Today's date as `YYYY-MM-DD`: the local one from `date`, else UTC from the clock.
pub fn today() -> String {
    command("date", &["+%Y-%m-%d"]).filter(|d| d.len() == 10).unwrap_or_else(|| {
        let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let (y, m, d) = civil_from_days((secs / 86_400) as i64);
        format!("{y:04}-{m:02}-{d:02}")
    })
}

/// Days since 1970-01-01 to a calendar date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_from_day_counts() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_000), (2024, 10, 4));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
    }

    #[test]
    fn reads_linux_files() {
        let cpu = "processor\t: 0\nmodel name\t: AMD EPYC 7763 64-Core Processor\nmodel name\t: other\n";
        assert_eq!(cpuinfo_model(cpu).as_deref(), Some("AMD EPYC 7763 64-Core Processor"));
        assert_eq!(cpuinfo_model("nothing"), None);
        assert_eq!(
            os_pretty_name("NAME=x\nPRETTY_NAME=\"Ubuntu 24.04 LTS\"\n").as_deref(),
            Some("Ubuntu 24.04 LTS")
        );
        assert_eq!(meminfo_gb("MemTotal:       16384000 kB\n").as_deref(), Some("16 GB"));
    }

    #[test]
    fn a_machine_always_has_a_name() {
        let m = machine();
        assert!(!m.cpu.is_empty() && !m.os.is_empty());
    }
}

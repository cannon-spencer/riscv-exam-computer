//! Board stats for the heartbeat, read from procfs and sysfs. Every reading is
//! optional: a missing file or an odd format leaves that field empty rather
//! than failing the heartbeat.

use std::fs;
use std::net::UdpSocket;

use proto::MemStats;

/// Hottest sensor, in °C. Thermal zones where the kernel has them, plus hwmon
/// inputs: the JH7110 on the Orange Pi RV exposes its sensor (sfctemp) only
/// through hwmon.
pub fn cpu_temp_c() -> Option<f32> {
    let zones = sensor_files("/sys/class/thermal", "thermal_zone", |n| n == "temp");
    let hwmon = sensor_files("/sys/class/hwmon", "hwmon", |n| n.starts_with("temp") && n.ends_with("_input"));
    zones
        .chain(hwmon)
        .filter_map(|p| fs::read_to_string(p).ok())
        .filter_map(|t| parse_millidegrees(&t))
        .reduce(f32::max)
}

/// Files matching `file` inside each `class/<prefix>*` device directory.
fn sensor_files(class: &str, prefix: &str, file: fn(&str) -> bool) -> impl Iterator<Item = std::path::PathBuf> {
    let prefix = prefix.to_string();
    fs::read_dir(class)
        .into_iter()
        .flatten()
        .flatten()
        .filter(move |d| d.file_name().to_string_lossy().starts_with(&prefix))
        .filter_map(|d| fs::read_dir(d.path()).ok())
        .flatten()
        .flatten()
        .filter(move |f| file(&f.file_name().to_string_lossy()))
        .map(|f| f.path())
}

pub fn load() -> Option<[f32; 3]> {
    parse_loadavg(&fs::read_to_string("/proc/loadavg").ok()?)
}

pub fn mem() -> Option<MemStats> {
    parse_meminfo(&fs::read_to_string("/proc/meminfo").ok()?)
}

pub fn uptime_s() -> Option<u64> {
    let text = fs::read_to_string("/proc/uptime").ok()?;
    text.split_whitespace().next()?.parse::<f64>().ok().map(|s| s as u64)
}

/// Address of the interface that routes to the internet. Connecting a UDP
/// socket only picks a route; no packet is sent.
pub fn ip() -> Option<String> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("1.1.1.1:80").ok()?;
    Some(s.local_addr().ok()?.ip().to_string())
}

fn parse_millidegrees(text: &str) -> Option<f32> {
    let m: f32 = text.trim().parse().ok()?;
    // Sensors that are off report 0 or nonsense; keep plausible readings only.
    let c = m / 1000.0;
    (c > 0.0 && c < 150.0).then(|| (c * 10.0).round() / 10.0)
}

fn parse_loadavg(text: &str) -> Option<[f32; 3]> {
    let mut it = text.split_whitespace().map(|v| v.parse::<f32>().ok());
    Some([it.next()??, it.next()??, it.next()??])
}

fn parse_meminfo(text: &str) -> Option<MemStats> {
    let field = |name: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(name)?.strip_prefix(':'))
            .and_then(|v| v.split_whitespace().next()?.parse::<u64>().ok())
    };
    Some(MemStats { total_kb: field("MemTotal")?, available_kb: field("MemAvailable")? })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_proc_files() {
        assert_eq!(parse_loadavg("0.42 0.31 0.25 1/213 4021\n"), Some([0.42, 0.31, 0.25]));
        assert_eq!(parse_loadavg("garbage"), None);
        let mi = "MemTotal:        2009100 kB\nMemFree:          738000 kB\nMemAvailable:    1468000 kB\n";
        assert_eq!(parse_meminfo(mi), Some(MemStats { total_kb: 2_009_100, available_kb: 1_468_000 }));
        assert_eq!(parse_meminfo("MemTotal: 1 kB\n"), None);
        assert_eq!(parse_millidegrees("45123\n"), Some(45.1));
        assert_eq!(parse_millidegrees("0\n"), None);
        assert_eq!(parse_millidegrees("-40000\n"), None);
    }
}

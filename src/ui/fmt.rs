//! Human-friendly number, size and time formatting.

use crate::api::Item;
use chrono::{Local, TimeZone};

pub fn bytes(v: f64) -> String {
    scaled(v, 1024.0, &["B", "KiB", "MiB", "GiB", "TiB", "PiB"])
}

pub fn bits(v: f64) -> String {
    scaled(v, 1000.0, &["bps", "Kbps", "Mbps", "Gbps", "Tbps"])
}

fn scaled(mut v: f64, step: f64, units: &[&str]) -> String {
    let mut i = 0;
    while v.abs() >= step && i < units.len() - 1 {
        v /= step;
        i += 1;
    }
    if i == 0 || v >= 100.0 { format!("{v:.0} {}", units[i]) } else { format!("{v:.1} {}", units[i]) }
}

/// Compact duration: 45s, 12m, 3h 5m, 4d 2h.
pub fn duration(secs: i64) -> String {
    let s = secs.max(0);
    let (d, h, m) = (s / 86400, (s % 86400) / 3600, (s % 3600) / 60);
    match (d, h, m) {
        (0, 0, 0) => format!("{s}s"),
        (0, 0, m) => format!("{m}m"),
        (0, h, 0) => format!("{h}h"),
        (0, h, m) => format!("{h}h {m}m"),
        (d, 0, _) => format!("{d}d"),
        (d, h, _) => format!("{d}d {h}h"),
    }
}

pub fn age(clock: i64) -> String {
    duration(Local::now().timestamp() - clock)
}

pub fn clock(ts: i64) -> String {
    match Local.timestamp_opt(ts, 0).single() {
        Some(t) if Local::now().timestamp() - ts < 86400 => t.format("%H:%M:%S").to_string(),
        Some(t) => t.format("%b %d %H:%M").to_string(),
        None => "-".into(),
    }
}

pub fn hhmm(ts: f64) -> String {
    Local.timestamp_opt(ts as i64, 0).single().map(|t| t.format("%H:%M").to_string()).unwrap_or_default()
}

pub fn pct(v: Option<f64>) -> String {
    match v {
        Some(v) if v >= 99.95 => "100%".into(),
        Some(v) if v >= 10.0 => format!("{v:.0}%"),
        Some(v) => format!("{v:.1}%"),
        None => "-".into(),
    }
}

pub fn number(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{v:.0}")
    } else if v.abs() >= 100.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    }
}

/// Render an item's last value the way the Zabbix frontend would.
pub fn item_value(item: &Item) -> String {
    if item.lastclock == "0" {
        return "-".into();
    }
    if !item.is_numeric() {
        let v = item.lastvalue.replace(['\n', '\r'], " ");
        return if v.is_empty() { "\"\"".into() } else { v };
    }
    let Ok(v) = item.lastvalue.parse::<f64>() else {
        return item.lastvalue.clone();
    };
    with_units(v, &item.units)
}

pub fn with_units(v: f64, units: &str) -> String {
    match units {
        "B" => bytes(v),
        "Bps" => format!("{}/s", bytes(v)),
        "bps" => bits(v),
        "uptime" | "s" => duration(v as i64),
        "unixtime" => clock(v as i64),
        "%" => format!("{v:.1}%"),
        "" => number(v),
        u => format!("{} {u}", number(v)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(bytes(8393289728.0), "7.8 GiB");
        assert_eq!(bits(2592.0), "2.6 Kbps");
        assert_eq!(duration(337), "5m");
        assert_eq!(duration(3 * 86400 + 7200 + 60), "3d 2h");
        assert_eq!(pct(Some(1.7359)), "1.7%");
        assert_eq!(number(10.0), "10");
    }
}

//! View-friendly data derived from raw API responses.

use crate::api::Item;
use chrono::{DateTime, Local};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    NotClassified = 0,
    Information = 1,
    Warning = 2,
    Average = 3,
    High = 4,
    Disaster = 5,
}

impl Severity {
    pub const ALL: [Severity; 6] = [Severity::NotClassified, Severity::Information, Severity::Warning, Severity::Average, Severity::High, Severity::Disaster];

    pub fn from_code(code: &str) -> Self {
        match code.parse::<u8>().unwrap_or(0) {
            1 => Self::Information,
            2 => Self::Warning,
            3 => Self::Average,
            4 => Self::High,
            5 => Self::Disaster,
            _ => Self::NotClassified,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
            "not_classified" | "nc" => Some(Self::NotClassified),
            "information" | "info" => Some(Self::Information),
            "warning" | "warn" => Some(Self::Warning),
            "average" | "avg" => Some(Self::Average),
            "high" => Some(Self::High),
            "disaster" => Some(Self::Disaster),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::NotClassified => "Not classified",
            Self::Information => "Information",
            Self::Warning => "Warning",
            Self::Average => "Average",
            Self::High => "High",
            Self::Disaster => "Disaster",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Self::NotClassified => "N/C",
            Self::Information => "INFO",
            Self::Warning => "WARN",
            Self::Average => "AVG",
            Self::High => "HIGH",
            Self::Disaster => "DSTR",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Up,
    Down,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct HostRow {
    pub id: String,
    pub name: String,
    pub groups: Vec<String>,
    pub address: String,
    pub enabled: bool,
    pub maintenance: bool,
    pub availability: Availability,
    pub error: String,
    pub cpu: Option<f64>,
    pub mem: Option<f64>,
    pub load: Option<f64>,
    pub cpus: Option<f64>,
    pub uptime: Option<f64>,
    pub net_in: Option<f64>,
    pub net_out: Option<f64>,
    pub disk: Option<f64>,
    /// (unix seconds, percent) for the last ~30 minutes.
    pub cpu_hist: Vec<(f64, f64)>,
    pub mem_hist: Vec<(f64, f64)>,
    pub problems: [usize; 6],
}

impl HostRow {
    pub fn worst(&self) -> Option<Severity> {
        Severity::ALL.iter().rev().copied().find(|s| self.problems[*s as usize] > 0)
    }

    pub fn problem_count(&self) -> usize {
        self.problems.iter().sum()
    }
}

#[derive(Debug, Clone)]
pub struct Problem {
    pub eventid: String,
    pub name: String,
    pub severity: Severity,
    pub clock: i64,
    pub acknowledged: bool,
    pub suppressed: bool,
    pub hostid: String,
    pub host: String,
    pub opdata: String,
    pub tags: Vec<String>,
    pub manual_close: bool,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub hosts: Vec<HostRow>,
    pub problems: Vec<Problem>,
    pub fetched_at: DateTime<Local>,
    pub latency_ms: u128,
}

/// (unix seconds, value) samples.
pub type Series = Vec<(f64, f64)>;

#[derive(Debug, Clone)]
pub struct HostDetail {
    pub hostid: String,
    pub range: i64,
    pub cpu: Series,
    pub mem: Series,
    pub load: Series,
    pub net_in: Series,
    pub net_out: Series,
    pub disks: Vec<(String, f64)>,
    pub facts: Vec<(String, String)>,
    pub items: Vec<Item>,
}

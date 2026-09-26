//! Application state and keyboard handling. Rendering lives in `ui`.

use crate::config::{Config, Profile};
use crate::model::*;
use crate::theme::{self, Theme};
use crate::worker::{Cmd, Event};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::time::{Instant, SystemTime};
use tokio::sync::mpsc::UnboundedSender;

pub const RANGES: [(i64, &str); 5] = [(15 * 60, "15m"), (3600, "1h"), (6 * 3600, "6h"), (24 * 3600, "24h"), (7 * 24 * 3600, "7d")];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Dashboard,
    Hosts,
    Problems,
    Host,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostTab {
    Graphs,
    Items,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostSort {
    Name,
    Problems,
    Cpu,
    Memory,
}

impl HostSort {
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Problems => "problems",
            Self::Cpu => "cpu",
            Self::Memory => "memory",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Name => Self::Problems,
            Self::Problems => Self::Cpu,
            Self::Cpu => Self::Memory,
            Self::Memory => Self::Name,
        }
    }
}

pub enum Modal {
    Help,
    Ack { eventids: Vec<String>, title: String, input: String, close: bool, can_close: bool },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Ok,
    Error,
    Problem(Severity),
}

pub struct Toast {
    pub kind: ToastKind,
    pub title: String,
    pub body: String,
    pub born: Instant,
}

pub struct App {
    pub config: Config,
    pub profile: Profile,
    pub config_path: PathBuf,
    pub theme: Theme,
    theme_name: String,
    theme_sig: Option<(PathBuf, Option<SystemTime>)>,
    cmd: UnboundedSender<Cmd>,

    pub version: Option<String>,
    pub error: Option<String>,
    pub snapshot: Option<Snapshot>,
    pub loading: bool,

    pub view: View,
    pub back_to: View,
    pub modal: Option<Modal>,
    pub toasts: VecDeque<Toast>,

    pub hosts_state: TableState,
    pub problems_state: TableState,
    pub items_state: TableState,
    pub host_filter: String,
    pub problem_filter: String,
    pub item_filter: String,
    pub editing_filter: bool,
    pub host_sort: HostSort,
    pub min_severity: Severity,
    pub hide_acked: bool,
    pub dash_mem: bool,

    pub detail_host: Option<String>,
    pub detail: Option<HostDetail>,
    pub range_idx: usize,
    pub host_tab: HostTab,

    seen_events: HashSet<String>,
    notify_min: Severity,
    pub frame: u64,
    pub quit: bool,
}

impl App {
    pub fn new(config: Config, profile: Profile, config_path: PathBuf, cmd: UnboundedSender<Cmd>) -> Self {
        let theme_name = config.theme.clone();
        let notify_min = Severity::from_name(&config.notifications.min_severity).unwrap_or(Severity::Average);
        let mut app = Self {
            theme: Theme::load(&theme_name),
            theme_sig: theme::omarchy_signature(),
            theme_name,
            config,
            profile,
            config_path,
            cmd,
            version: None,
            error: None,
            snapshot: None,
            loading: true,
            view: View::Dashboard,
            back_to: View::Hosts,
            modal: None,
            toasts: VecDeque::new(),
            hosts_state: TableState::default().with_selected(0),
            problems_state: TableState::default().with_selected(0),
            items_state: TableState::default().with_selected(0),
            host_filter: String::new(),
            problem_filter: String::new(),
            item_filter: String::new(),
            editing_filter: false,
            host_sort: HostSort::Problems,
            min_severity: Severity::NotClassified,
            hide_acked: false,
            dash_mem: false,
            detail_host: None,
            detail: None,
            range_idx: 1,
            host_tab: HostTab::Graphs,
            seen_events: HashSet::new(),
            notify_min,
            frame: 0,
            quit: false,
        };
        app.send(Cmd::Refresh);
        app
    }

    fn send(&mut self, cmd: Cmd) {
        self.loading = true;
        let _ = self.cmd.send(cmd);
    }

    pub fn toast(&mut self, kind: ToastKind, title: impl Into<String>, body: impl Into<String>) {
        self.toasts.push_front(Toast { kind, title: title.into(), body: body.into(), born: Instant::now() });
        self.toasts.truncate(4);
    }

    /// Called every UI tick (~250ms): expire toasts, follow Omarchy theme swaps.
    pub fn on_tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        self.toasts.retain(|t| t.born.elapsed().as_secs() < 6);
        if self.frame.is_multiple_of(8) && (self.theme_name == "auto" || self.theme_name == "omarchy") {
            let sig = theme::omarchy_signature();
            if sig != self.theme_sig {
                self.theme_sig = sig;
                self.theme = Theme::load(&self.theme_name);
                let name = self.theme.name.clone();
                self.toast(ToastKind::Info, "Theme", name);
            }
        }
    }

    pub fn on_event(&mut self, event: Event) {
        match event {
            Event::Connected { version } => {
                self.version = Some(version);
                self.error = None;
            }
            Event::Snapshot(snap) => {
                self.loading = false;
                self.error = None;
                self.announce_new_problems(&snap);
                self.snapshot = Some(*snap);
                self.clamp_selection();
                // Keep an open host page as live as the rest of the app.
                if self.view == View::Host {
                    self.request_detail();
                }
            }
            Event::Host(detail) => {
                self.loading = false;
                if self.detail_host.as_deref() == Some(detail.hostid.as_str()) {
                    self.detail = Some(*detail);
                }
            }
            Event::Acked(n) => {
                self.toast(ToastKind::Ok, "Acknowledged", format!("{n} problem{}", if n == 1 { "" } else { "s" }));
            }
            Event::Error(e) => {
                self.loading = false;
                if self.error.as_deref() != Some(e.as_str()) {
                    self.toast(ToastKind::Error, "Zabbix API", e.clone());
                }
                self.error = Some(e);
            }
        }
    }

    fn announce_new_problems(&mut self, snap: &Snapshot) {
        let first = self.snapshot.is_none();
        let min = self.notify_min;
        let fresh: Vec<&Problem> =
            snap.problems.iter().filter(|p| !self.seen_events.contains(&p.eventid) && !p.suppressed && p.severity >= min).take(3).collect();
        self.seen_events = snap.problems.iter().map(|p| p.eventid.clone()).collect();
        if first {
            return;
        }
        for p in fresh {
            self.toast(ToastKind::Problem(p.severity), format!("{} · {}", p.severity.label(), p.host), p.name.clone());
            if self.config.notifications.desktop {
                desktop_notify(p);
            }
        }
    }

    // ---- derived lists -------------------------------------------------

    pub fn hosts(&self) -> Vec<&HostRow> {
        let Some(snap) = &self.snapshot else {
            return vec![];
        };
        let needle = self.host_filter.to_lowercase();
        let mut hosts: Vec<&HostRow> = snap
            .hosts
            .iter()
            .filter(|h| {
                needle.is_empty()
                    || h.name.to_lowercase().contains(&needle)
                    || h.address.to_lowercase().contains(&needle)
                    || h.groups.iter().any(|g| g.to_lowercase().contains(&needle))
            })
            .collect();
        let desc = |a: Option<f64>, b: Option<f64>| b.unwrap_or(-1.0).total_cmp(&a.unwrap_or(-1.0));
        match self.host_sort {
            HostSort::Name => hosts.sort_by_key(|h| h.name.to_lowercase()),
            HostSort::Problems => hosts.sort_by(|a, b| b.worst().cmp(&a.worst()).then(b.problem_count().cmp(&a.problem_count())).then(a.name.cmp(&b.name))),
            HostSort::Cpu => hosts.sort_by(|a, b| desc(a.cpu, b.cpu)),
            HostSort::Memory => hosts.sort_by(|a, b| desc(a.mem, b.mem)),
        }
        hosts
    }

    pub fn problems(&self) -> Vec<&Problem> {
        let Some(snap) = &self.snapshot else {
            return vec![];
        };
        let needle = self.problem_filter.to_lowercase();
        snap.problems
            .iter()
            .filter(|p| p.severity >= self.min_severity)
            .filter(|p| !(self.hide_acked && p.acknowledged))
            .filter(|p| {
                needle.is_empty()
                    || p.name.to_lowercase().contains(&needle)
                    || p.host.to_lowercase().contains(&needle)
                    || p.tags.iter().any(|t| t.to_lowercase().contains(&needle))
            })
            .collect()
    }

    pub fn items(&self) -> Vec<&crate::api::Item> {
        let Some(d) = &self.detail else { return vec![] };
        let needle = self.item_filter.to_lowercase();
        d.items.iter().filter(|i| needle.is_empty() || i.name.to_lowercase().contains(&needle) || i.key_.to_lowercase().contains(&needle)).collect()
    }

    pub fn detail_row(&self) -> Option<&HostRow> {
        let id = self.detail_host.as_deref()?;
        self.snapshot.as_ref()?.hosts.iter().find(|h| h.id == id)
    }

    pub fn range_label(&self) -> &'static str {
        RANGES[self.range_idx].1
    }

    pub fn filter_mut(&mut self) -> Option<&mut String> {
        match self.view {
            View::Hosts => Some(&mut self.host_filter),
            View::Problems => Some(&mut self.problem_filter),
            View::Host if self.host_tab == HostTab::Items => Some(&mut self.item_filter),
            _ => None,
        }
    }

    fn clamp_selection(&mut self) {
        let (h, p, i) = (self.hosts().len(), self.problems().len(), self.items().len());
        for (state, len) in [(&mut self.hosts_state, h), (&mut self.problems_state, p), (&mut self.items_state, i)] {
            let sel = state.selected().unwrap_or(0);
            state.select(Some(sel.min(len.saturating_sub(1))));
        }
    }

    // ---- input ---------------------------------------------------------

    pub fn on_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if self.modal.is_some() {
            self.on_modal_key(key);
            return;
        }
        if self.editing_filter {
            self.on_filter_key(key);
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('?') => self.modal = Some(Modal::Help),
            KeyCode::Char('1') => self.view = View::Dashboard,
            KeyCode::Char('2') => self.view = View::Hosts,
            KeyCode::Char('3') => self.view = View::Problems,
            KeyCode::Tab if self.view != View::Host => {
                self.view = match self.view {
                    View::Dashboard => View::Hosts,
                    View::Hosts => View::Problems,
                    _ => View::Dashboard,
                }
            }
            KeyCode::BackTab if self.view != View::Host => {
                self.view = match self.view {
                    View::Dashboard => View::Problems,
                    View::Problems => View::Hosts,
                    _ => View::Dashboard,
                }
            }
            KeyCode::Char('r') => self.refresh(),
            KeyCode::Char('t') => {
                self.theme_name = Theme::next_name(&self.theme.name);
                self.theme = Theme::load(&self.theme_name);
                let name = self.theme.name.clone();
                self.toast(ToastKind::Info, "Theme", name);
            }
            KeyCode::Char('o') => self.open_in_browser(),
            KeyCode::Char('/') if self.filter_mut().is_some() => self.editing_filter = true,
            KeyCode::Esc => self.escape(),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::PageDown => self.move_selection(10),
            KeyCode::PageUp => self.move_selection(-10),
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => self.move_selection(10),
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => self.move_selection(-10),
            KeyCode::Char('g') | KeyCode::Home => self.move_selection(isize::MIN / 2),
            KeyCode::Char('G') | KeyCode::End => self.move_selection(isize::MAX / 2),
            _ => self.on_view_key(key),
        }
    }

    fn on_view_key(&mut self, key: KeyEvent) {
        match (self.view, key.code) {
            (View::Hosts | View::Dashboard, KeyCode::Enter | KeyCode::Char('l')) => {
                if let Some(id) = self.hosts().get(self.hosts_state.selected().unwrap_or(0)).map(|h| h.id.clone()) {
                    let from = self.view;
                    self.open_host(id, from);
                }
            }
            (View::Dashboard, KeyCode::Char('m')) => self.dash_mem = !self.dash_mem,
            (View::Hosts, KeyCode::Char('s')) => {
                self.host_sort = self.host_sort.next();
                self.hosts_state.select(Some(0));
            }
            (View::Problems, KeyCode::Enter | KeyCode::Char('l')) => {
                if let Some(id) = self.selected_problem().map(|p| p.hostid.clone()).filter(|id| !id.is_empty()) {
                    self.open_host(id, View::Problems);
                }
            }
            (View::Problems, KeyCode::Char('a')) => {
                if let Some(p) = self.selected_problem() {
                    self.modal = Some(Modal::Ack {
                        eventids: vec![p.eventid.clone()],
                        title: format!("{} · {}", p.host, p.name),
                        input: String::new(),
                        close: false,
                        can_close: p.manual_close,
                    });
                }
            }
            (View::Problems, KeyCode::Char('+') | KeyCode::Char('s')) => {
                let idx = (self.min_severity as usize + 1) % 6;
                self.min_severity = Severity::ALL[idx];
                self.problems_state.select(Some(0));
            }
            (View::Problems, KeyCode::Char('-')) => {
                let idx = (self.min_severity as usize + 5) % 6;
                self.min_severity = Severity::ALL[idx];
                self.problems_state.select(Some(0));
            }
            (View::Problems, KeyCode::Char('h')) => {
                self.hide_acked = !self.hide_acked;
                self.problems_state.select(Some(0));
            }
            (View::Host, KeyCode::Char('[')) => self.shift_range(-1),
            (View::Host, KeyCode::Char(']')) => self.shift_range(1),
            (View::Host, KeyCode::Tab | KeyCode::Char('i')) => {
                self.host_tab = if self.host_tab == HostTab::Graphs { HostTab::Items } else { HostTab::Graphs };
            }
            (View::Host, KeyCode::Char('h') | KeyCode::Backspace) => self.escape(),
            _ => {}
        }
    }

    fn on_filter_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                if let Some(f) = self.filter_mut() {
                    f.clear();
                }
                self.editing_filter = false;
            }
            KeyCode::Enter => self.editing_filter = false,
            KeyCode::Backspace => {
                if let Some(f) = self.filter_mut() {
                    f.pop();
                }
            }
            KeyCode::Down => self.move_selection(1),
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Char(c) => {
                if let Some(f) = self.filter_mut() {
                    f.push(c);
                }
            }
            _ => {}
        }
        for state in [&mut self.hosts_state, &mut self.problems_state, &mut self.items_state] {
            if state.selected().is_none() {
                state.select(Some(0));
            }
        }
        self.clamp_selection();
    }

    fn on_modal_key(&mut self, key: KeyEvent) {
        let Some(modal) = self.modal.as_mut() else {
            return;
        };
        match modal {
            Modal::Help => {
                if matches!(key.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Enter) {
                    self.modal = None;
                }
            }
            Modal::Ack { eventids, input, close, can_close, .. } => match key.code {
                KeyCode::Esc => self.modal = None,
                KeyCode::Tab if *can_close => *close = !*close,
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(c) => input.push(c),
                KeyCode::Enter => {
                    let cmd = Cmd::Ack { eventids: std::mem::take(eventids), message: input.trim().to_string(), close: *close };
                    self.modal = None;
                    self.send(cmd);
                }
                _ => {}
            },
        }
    }

    fn escape(&mut self) {
        if self.view == View::Host {
            self.view = self.back_to;
            return;
        }
        if let Some(f) = self.filter_mut() {
            f.clear();
        }
    }

    fn refresh(&mut self) {
        self.send(Cmd::Refresh);
        if self.view == View::Host {
            self.request_detail();
        }
    }

    fn open_host(&mut self, hostid: String, from: View) {
        if self.detail_host.as_deref() != Some(hostid.as_str()) {
            self.detail = None;
            self.item_filter.clear();
            self.items_state.select(Some(0));
        }
        self.detail_host = Some(hostid);
        self.back_to = from;
        self.view = View::Host;
        self.host_tab = HostTab::Graphs;
        self.request_detail();
    }

    pub fn request_detail(&mut self) {
        if let Some(hostid) = self.detail_host.clone() {
            self.send(Cmd::Host { hostid, range: RANGES[self.range_idx].0 });
        }
    }

    fn shift_range(&mut self, delta: isize) {
        let next = (self.range_idx as isize + delta).clamp(0, RANGES.len() as isize - 1) as usize;
        if next != self.range_idx {
            self.range_idx = next;
            self.request_detail();
        }
    }

    pub fn selected_problem(&self) -> Option<&Problem> {
        self.problems().get(self.problems_state.selected().unwrap_or(0)).copied()
    }

    fn move_selection(&mut self, delta: isize) {
        let (state, len) = match self.view {
            View::Hosts | View::Dashboard => {
                let len = self.hosts().len();
                (&mut self.hosts_state, len)
            }
            View::Problems => {
                let len = self.problems().len();
                (&mut self.problems_state, len)
            }
            View::Host if self.host_tab == HostTab::Items => {
                let len = self.items().len();
                (&mut self.items_state, len)
            }
            View::Host => return,
        };
        if len == 0 {
            return;
        }
        let cur = state.selected().unwrap_or(0) as isize;
        state.select(Some(cur.saturating_add(delta).clamp(0, len as isize - 1) as usize));
    }

    fn open_in_browser(&mut self) {
        let base = self.profile.web_url();
        let url = match self.view {
            View::Dashboard => format!("{base}/zabbix.php?action=dashboard.view"),
            View::Hosts => match self.hosts().get(self.hosts_state.selected().unwrap_or(0)) {
                Some(h) => format!("{base}/zabbix.php?action=latest.view&hostids%5B%5D={}", h.id),
                None => format!("{base}/zabbix.php?action=host.view"),
            },
            View::Problems => match self.selected_problem() {
                Some(p) => format!("{base}/zabbix.php?action=problem.view&hostids%5B%5D={}", p.hostid),
                None => format!("{base}/zabbix.php?action=problem.view"),
            },
            View::Host => format!("{base}/zabbix.php?action=latest.view&hostids%5B%5D={}", self.detail_host.clone().unwrap_or_default()),
        };
        let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
        match spawn_detached(std::process::Command::new(opener).arg(&url)) {
            Ok(()) => self.toast(ToastKind::Info, "Opened in browser", url),
            Err(e) => self.toast(ToastKind::Error, "Could not open browser", e.to_string()),
        }
    }
}

fn desktop_notify(p: &Problem) {
    let title = format!("{} · {}", p.severity.label(), p.host);
    let urgency = if p.severity >= Severity::High { "critical" } else { "normal" };
    // A missing notifier is fine; the in-app toast already fired.
    let _ = if cfg!(target_os = "macos") {
        let script = format!("display notification {:?} with title \"zabterm\" subtitle {:?}", p.name, title);
        spawn_detached(std::process::Command::new("osascript").args(["-e", &script]))
    } else {
        spawn_detached(std::process::Command::new("notify-send").args(["-a", "zabterm", "-u", urgency, &title, &p.name]))
    };
}

/// Run a helper without touching the TUI's terminal, and reap it so it
/// doesn't linger as a zombie.
fn spawn_detached(cmd: &mut std::process::Command) -> std::io::Result<()> {
    use std::process::Stdio;
    let mut child = cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
    std::thread::spawn(move || child.wait());
    Ok(())
}

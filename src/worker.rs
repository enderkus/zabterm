//! Background task that owns the API client. The UI never blocks on the
//! network: it sends `Cmd`s and receives `Event`s over channels.

use crate::api::{Client, Item};
use crate::config::Profile;
use crate::model::*;
use anyhow::Result;
use chrono::Local;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

const SPARK_WINDOW: i64 = 30 * 60;

const CPU: &str = "system.cpu.util";
const MEM: &str = "vm.memory.util";
const LOAD: &str = "system.cpu.load[all,avg1]";
const UPTIME: &str = "system.uptime";
const NCPU: &str = "system.cpu.num";

pub enum Cmd {
    Refresh,
    Host { hostid: String, range: i64 },
    Ack { eventids: Vec<String>, message: String, close: bool },
}

pub enum Event {
    Connected { version: String },
    Snapshot(Box<Snapshot>),
    Host(Box<HostDetail>),
    Acked(usize),
    Error(String),
}

pub fn spawn(profile: Profile, interval: u64, events: UnboundedSender<Event>) -> UnboundedSender<Cmd> {
    let (tx, rx) = unbounded_channel();
    tokio::spawn(run(profile, interval.max(2), rx, events));
    tx
}

async fn run(profile: Profile, interval: u64, mut rx: UnboundedReceiver<Cmd>, events: UnboundedSender<Event>) {
    let mut client: Option<Client> = None;
    let mut series = SeriesStore::default();
    let mut ticker = tokio::time::interval(Duration::from_secs(interval));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        let cmd = tokio::select! {
            _ = ticker.tick() => Cmd::Refresh,
            cmd = rx.recv() => match cmd { Some(c) => c, None => return },
        };

        if client.is_none() {
            match connect(&profile).await {
                Ok((c, version)) => {
                    client = Some(c);
                    let _ = events.send(Event::Connected { version });
                }
                Err(e) => {
                    let _ = events.send(Event::Error(describe(&e)));
                    continue;
                }
            }
        }
        let api = client.as_ref().expect("connected above");

        let result = match cmd {
            Cmd::Refresh => snapshot(api, &mut series).await.map(|s| Event::Snapshot(Box::new(s))),
            Cmd::Host { hostid, range } => host_detail(api, &hostid, range).await.map(|d| Event::Host(Box::new(d))),
            Cmd::Ack { eventids, message, close } => {
                let n = eventids.len();
                match api.acknowledge(&eventids, &message, close).await {
                    Ok(()) => {
                        let _ = events.send(Event::Acked(n));
                        snapshot(api, &mut series).await.map(|s| Event::Snapshot(Box::new(s)))
                    }
                    Err(e) => Err(e),
                }
            }
        };
        if let Err(e) = &result {
            // Expired login sessions come back as auth errors; log in again next time.
            let msg = format!("{e:#}").to_lowercase();
            if msg.contains("not authorized") || msg.contains("session") || msg.contains("re-login") {
                client = None;
            }
        }
        let _ = events.send(result.unwrap_or_else(|e| Event::Error(describe(&e))));
    }
}

/// "what we were doing: why it failed", skipping reqwest's noisy middle layers.
fn describe(e: &anyhow::Error) -> String {
    let root = e.root_cause().to_string();
    let top = e.to_string();
    if top == root { top } else { format!("{top}: {root}") }
}

async fn connect(profile: &Profile) -> Result<(Client, String)> {
    let client = Client::connect(profile).await?;
    let version = client.version().await?;
    Ok((client, version))
}

/// Rolling per-host CPU/memory buffers so the dashboard can draw sparklines
/// without re-reading history on every poll.
#[derive(Default)]
struct SeriesStore {
    seeded: HashSet<String>,
    data: HashMap<String, VecDeque<(i64, f64)>>,
}

impl SeriesStore {
    fn push(&mut self, itemid: &str, clock: i64, value: f64) {
        let buf = self.data.entry(itemid.to_string()).or_default();
        if buf.back().is_none_or(|(c, _)| clock > *c) {
            buf.push_back((clock, value));
        }
        let cutoff = Local::now().timestamp() - SPARK_WINDOW;
        while buf.front().is_some_and(|(c, _)| *c < cutoff) {
            buf.pop_front();
        }
    }

    fn get(&self, itemid: &str) -> Vec<(f64, f64)> {
        self.data.get(itemid).map(|b| b.iter().map(|(c, v)| (*c as f64, *v)).collect()).unwrap_or_default()
    }
}

async fn snapshot(api: &Client, series: &mut SeriesStore) -> Result<Snapshot> {
    let started = Instant::now();
    // The net/disk prefix search has no hostids filter, so on instances with
    // many thousands of hosts it can run far longer than the other three
    // calls (or hit a reverse-proxy timeout) and return 5xx/timeout errors
    // well before it succeeds. Those stats are a bonus for the dashboard, not
    // required for it to render, so a failure here must not take down hosts,
    // problems and the core CPU/mem items with it.
    let (hosts, raw_problems, items, extra) = tokio::join!(
        api.hosts(),
        api.problems(),
        api.items_by_key(None, &[CPU, MEM, LOAD, UPTIME, NCPU]),
        api.items_by_prefix(None, &["net.if.in[", "net.if.out[", "vfs.fs.dependent.size["]),
    );
    let (hosts, raw_problems, items) = (hosts?, raw_problems?, items?);
    let extra = extra.unwrap_or_default();

    let triggerids: Vec<String> = raw_problems.iter().map(|p| p.objectid.clone()).collect::<HashSet<_>>().into_iter().collect();
    let triggers = api.trigger_hosts(&triggerids).await?;
    let trigger_map: HashMap<&str, _> = triggers.iter().map(|t| (t.triggerid.as_str(), t)).collect();

    // Seed sparkline buffers from real history the first time we see an item.
    let unseeded: Vec<String> =
        items.iter().filter(|i| (i.key_ == CPU || i.key_ == MEM) && !series.seeded.contains(&i.itemid)).map(|i| i.itemid.clone()).collect();
    if !unseeded.is_empty() {
        let from = Local::now().timestamp() - SPARK_WINDOW;
        for p in api.history(&unseeded, 0, from).await.unwrap_or_default() {
            if let (Ok(c), Ok(v)) = (p.clock.parse(), p.value.parse()) {
                series.push(&p.itemid, c, v);
            }
        }
        series.seeded.extend(unseeded);
    }
    for i in items.iter().filter(|i| i.key_ == CPU || i.key_ == MEM) {
        if let (Some(v), Ok(c)) = (i.value(), i.lastclock.parse()) {
            series.push(&i.itemid, c, v);
        }
    }

    let mut by_host: HashMap<&str, Vec<&Item>> = HashMap::new();
    for i in items.iter().chain(extra.iter()) {
        by_host.entry(i.hostid.as_str()).or_default().push(i);
    }

    let mut problems: Vec<Problem> = raw_problems
        .into_iter()
        .map(|p| {
            let trig = trigger_map.get(p.objectid.as_str());
            let host = trig.and_then(|t| t.hosts.first());
            Problem {
                eventid: p.eventid,
                name: p.name,
                severity: Severity::from_code(&p.severity),
                clock: p.clock.parse().unwrap_or(0),
                acknowledged: p.acknowledged == "1",
                suppressed: p.suppressed == "1",
                hostid: host.map(|h| h.hostid.clone()).unwrap_or_default(),
                host: host.map(|h| h.name.clone()).unwrap_or_else(|| "?".into()),
                opdata: p.opdata,
                tags: p.tags.into_iter().map(|t| if t.value.is_empty() { t.tag } else { format!("{}:{}", t.tag, t.value) }).collect(),
                manual_close: trig.is_some_and(|t| t.manual_close == "1"),
            }
        })
        .collect();
    problems.sort_by(|a, b| b.severity.cmp(&a.severity).then(b.clock.cmp(&a.clock)));

    let hosts = hosts
        .into_iter()
        .map(|h| {
            let its = by_host.get(h.hostid.as_str()).cloned().unwrap_or_default();
            let find = |key: &str| its.iter().find(|i| i.key_ == key).copied();
            let val = |key: &str| find(key).and_then(Item::value);
            let hist = |key: &str| find(key).map(|i| series.get(&i.itemid)).unwrap_or_default();

            let mut counts = [0usize; 6];
            for p in problems.iter().filter(|p| p.hostid == h.hostid && !p.suppressed) {
                counts[p.severity as usize] += 1;
            }

            // Agent interface wins; otherwise whatever interface comes first.
            let iface = h.interfaces.iter().find(|i| i.kind == "1").or(h.interfaces.first());
            let passive = iface.map(|i| i.available.as_str()).unwrap_or("0");
            let availability = match (passive, h.active_available.as_str()) {
                ("2", _) | (_, "2") => Availability::Down,
                ("1", _) | (_, "1") => Availability::Up,
                _ => Availability::Unknown,
            };

            HostRow {
                name: h.name,
                groups: h.hostgroups.into_iter().map(|g| g.name).collect(),
                address: iface.map(|i| i.address()).unwrap_or_default(),
                enabled: h.status == "0",
                maintenance: h.maintenance_status == "1",
                availability,
                error: iface.map(|i| i.error.clone()).unwrap_or_default(),
                cpu: val(CPU),
                mem: val(MEM),
                load: val(LOAD),
                cpus: val(NCPU),
                uptime: val(UPTIME),
                net_in: sum_net(&its, "net.if.in["),
                net_out: sum_net(&its, "net.if.out["),
                disk: root_disk(&its),
                cpu_hist: hist(CPU),
                mem_hist: hist(MEM),
                problems: counts,
                id: h.hostid,
            }
        })
        .collect();

    Ok(Snapshot { hosts, problems, fetched_at: Local::now(), latency_ms: started.elapsed().as_millis() })
}

/// `net.if.in["eth0"]` style keys (no second parameter), loopback excluded.
fn traffic_iface<'a>(key: &'a str, prefix: &str) -> Option<&'a str> {
    let inner = key.strip_prefix(prefix)?.strip_suffix(']')?;
    if inner.contains(',') {
        return None;
    }
    let name = inner.trim_matches('"');
    (name != "lo").then_some(name)
}

fn sum_net(items: &[&Item], prefix: &str) -> Option<f64> {
    let vals: Vec<f64> = items.iter().filter(|i| traffic_iface(&i.key_, prefix).is_some()).filter_map(|i| i.value()).collect();
    (!vals.is_empty()).then(|| vals.iter().sum())
}

fn disk_mount(key: &str) -> Option<&str> {
    key.strip_prefix("vfs.fs.dependent.size[")?.strip_suffix(",pused]")
}

/// Root filesystem usage, or the fullest real mount if `/` is not monitored.
/// Container bind mounts under /etc are noise, so they are skipped.
fn root_disk(items: &[&Item]) -> Option<f64> {
    let mounts: Vec<(&str, f64)> = items.iter().filter_map(|i| Some((disk_mount(&i.key_)?, i.value()?))).filter(|(m, _)| !m.starts_with("/etc/")).collect();
    mounts.iter().find(|(m, _)| *m == "/").or_else(|| mounts.iter().max_by(|a, b| a.1.total_cmp(&b.1))).map(|(_, v)| *v)
}

async fn host_detail(api: &Client, hostid: &str, range: i64) -> Result<HostDetail> {
    let items = api.host_items(hostid).await?;
    let from = Local::now().timestamp() - range;

    let pick = |key: &str| items.iter().find(|i| i.key_ == key);
    let (net_in_item, net_out_item) = {
        let ifaces: Vec<&Item> = items.iter().filter(|i| traffic_iface(&i.key_, "net.if.in[").is_some()).collect();
        // Busiest interface is the interesting one.
        let busiest = ifaces.iter().max_by(|a, b| a.value().unwrap_or(0.0).total_cmp(&b.value().unwrap_or(0.0)));
        let name = busiest.and_then(|i| traffic_iface(&i.key_, "net.if.in[")).map(str::to_string);
        let out = name.as_ref().and_then(|n| items.iter().find(|i| traffic_iface(&i.key_, "net.if.out[") == Some(n.as_str())));
        (busiest.copied(), out)
    };

    let mut ids: HashMap<u8, Vec<String>> = HashMap::new();
    for item in [pick(CPU), pick(MEM), pick(LOAD), net_in_item, net_out_item].into_iter().flatten() {
        ids.entry(item.value_type.parse().unwrap_or(0)).or_default().push(item.itemid.clone());
    }
    let (floats, uints) = tokio::try_join!(
        api.history(ids.get(&0).map(Vec::as_slice).unwrap_or_default(), 0, from),
        api.history(ids.get(&3).map(Vec::as_slice).unwrap_or_default(), 3, from),
    )?;
    let mut hist: HashMap<String, Vec<(f64, f64)>> = HashMap::new();
    for p in floats.into_iter().chain(uints) {
        if let (Ok(c), Ok(v)) = (p.clock.parse::<f64>(), p.value.parse::<f64>()) {
            hist.entry(p.itemid).or_default().push((c, v));
        }
    }
    let series = |item: Option<&Item>| -> Series { item.and_then(|i| hist.get(&i.itemid).cloned()).unwrap_or_default() };

    let text = |key: &str| pick(key).map(|i| i.lastvalue.clone()).filter(|v| !v.is_empty());
    let mut facts = Vec::new();
    for (label, key) in [("OS", "system.sw.os"), ("Hostname", "system.hostname"), ("Arch", "system.sw.arch"), ("Agent", "agent.version")] {
        if let Some(v) = text(key) {
            facts.push((label.to_string(), v));
        }
    }
    if let Some(n) = pick(NCPU).and_then(Item::value) {
        facts.push(("CPUs".into(), format!("{n:.0}")));
    }
    if let Some(m) = pick("vm.memory.size[total]").and_then(Item::value) {
        facts.push(("Memory".into(), crate::ui::fmt::bytes(m)));
    }
    if let Some(u) = pick(UPTIME).and_then(Item::value) {
        facts.push(("Uptime".into(), crate::ui::fmt::duration(u as i64)));
    }

    let mut disks: Vec<(String, f64)> =
        items.iter().filter_map(|i| Some((disk_mount(&i.key_)?.to_string(), i.value()?))).filter(|(m, _)| !m.starts_with("/etc/")).collect();
    disks.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(HostDetail {
        hostid: hostid.to_string(),
        range,
        cpu: series(pick(CPU)),
        mem: series(pick(MEM)),
        load: series(pick(LOAD)),
        net_in: series(net_in_item),
        net_out: series(net_out_item),
        disks,
        facts,
        items,
    })
}

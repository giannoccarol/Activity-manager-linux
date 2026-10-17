//! Per-app network: per-socket TCP rates from `ss` (bytes acked/received), attributed to the snapshot
//! groups via the socket's cgroup (or the PID if `ss` does not report it).
use crate::cgroup;
use crate::model::{Section, Snapshot};
use crate::util::{read_raw, run};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NetApp {
    pub key: String,
    pub label: String,
    pub icon: Option<String>,
    pub section: Section,
    pub recv: f64,
    pub send: f64,
    pub conns: u32,
    pub remotes: Vec<String>,
}

pub struct NetApps {
    prev: HashMap<(String, String), (u64, u64)>,
    prev_t: Option<Instant>,
    cgs: HashMap<(i32, u64), String>,
    by_pid: bool,
}

struct Sock {
    tcp: bool,
    estab: bool,
    local: String,
    peer: String,
    cg: Option<String>,
    pid: Option<i32>,
    rx: u64,
    tx: u64,
}

#[derive(Default)]
struct Acc {
    recv: f64,
    send: f64,
    conns: u32,
    remotes: HashMap<String, (f64, u64)>,
}

fn num(s: &str, key: &str) -> Option<u64> {
    let rest = &s[s.find(key)? + key.len()..];
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Lines of `ss -tuineH`/`ss -tuinpH`: "netid state recv-q send-q local remote [extra]" and, for TCP, an indented line with the counters.
fn parse(out: &str) -> Vec<Sock> {
    let mut socks: Vec<Sock> = vec![];
    let mut open = false;
    for line in out.lines() {
        if line.starts_with([' ', '\t']) {
            if let Some(s) = socks.last_mut().filter(|s| open && s.tcp) {
                s.rx = num(line, "bytes_received:").unwrap_or(0);
                s.tx = num(line, "bytes_acked:").or_else(|| num(line, "bytes_sent:")).unwrap_or(0);
            }
            open = false;
            continue;
        }
        let f: Vec<&str> = line.split_whitespace().take(6).collect();
        open = f.len() == 6 && f[1] != "TIME-WAIT";
        if !open {
            continue;
        }
        let cg = line.find(" cgroup:").map(|i| line[i + 8..].split_whitespace().next().unwrap_or("").to_string());
        let pid = line.find("users:((").and_then(|i| num(&line[i..], "pid=")).map(|p| p as i32);
        socks.push(Sock { tcp: f[0] == "tcp", estab: f[1] == "ESTAB", local: f[4].into(), peer: f[5].into(), cg, pid, rx: 0, tx: 0 });
    }
    socks
}

/// Host and port from "1.2.3.4:443", "[2a00::1]:443", "[::ffff:1.2.3.4]:443", "10.0.0.1%wlan0:67".
fn split_addr(a: &str) -> (&str, &str) {
    let (host, port) = match a.strip_prefix('[') {
        Some(r) => r.rsplit_once("]:").unwrap_or((r.trim_end_matches(']'), "")),
        None => a.rsplit_once(':').unwrap_or((a, "")),
    };
    let host = host.split('%').next().unwrap_or(host);
    (host.strip_prefix("::ffff:").unwrap_or(host), port)
}

fn is_local(host: &str) -> bool {
    host == "::1" || host == "localhost" || host.starts_with("127.")
}

/// Path of the process's systemd unit (like the keys of non-app groups).
fn unit_path(pid: i32) -> String {
    read_raw(format!("/proc/{pid}/cgroup"))
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("0::").map(|p| cgroup::parse(p.trim()).unit_path)))
        .unwrap_or_default()
}

impl NetApps {
    pub fn new() -> Self {
        NetApps { prev: HashMap::new(), prev_t: None, cgs: HashMap::new(), by_pid: false }
    }

    pub fn sample(&mut self, snap: &Snapshot) -> Vec<NetApp> {
        let args: &[&str] = if self.by_pid { &["-tuinpH"] } else { &["-tuineH"] };
        let out = run("ss", args, Duration::from_secs(4));
        if out.trim().is_empty() {
            return vec![];
        }
        let socks = parse(&out);
        // ss versions without the cgroup field: fall back to PIDs (slower)
        if !self.by_pid && !socks.is_empty() && socks.iter().all(|s| s.cg.is_none()) {
            self.by_pid = true;
        }
        let now = Instant::now();
        let dt = self.prev_t.map(|t| now.duration_since(t).as_secs_f64()).filter(|d| *d > 0.2 && *d < 10.0);
        self.prev_t = Some(now);

        let mut pids: HashMap<i32, usize> = HashMap::new();
        let mut paths: HashMap<String, usize> = HashMap::new();
        let mut live = HashSet::new();
        for (i, g) in snap.groups.iter().enumerate() {
            let unit_group = g.key.starts_with('/');
            if unit_group {
                paths.entry(g.key.clone()).or_insert(i);
            }
            for p in &g.procs {
                pids.entry(p.pid).or_insert(i);
                if !unit_group && !self.by_pid {
                    let k = (p.pid, p.ctime);
                    live.insert(k);
                    let path = self.cgs.entry(k).or_insert_with(|| unit_path(p.pid));
                    if path.len() > 1 {
                        paths.entry(path.clone()).or_insert(i);
                    }
                }
            }
        }
        self.cgs.retain(|k, _| live.contains(k));

        let mut accs: HashMap<Option<usize>, Acc> = HashMap::new();
        let mut prev = HashMap::with_capacity(socks.len());
        for s in socks {
            let (host, port) = split_addr(&s.peer);
            if is_local(host) {
                continue;
            }
            let remote = if host.contains(':') { format!("[{host}]:{port}") } else { format!("{host}:{port}") };
            let gi = s
                .cg
                .as_deref()
                .and_then(|c| paths.get(&cgroup::parse(c).unit_path).copied())
                .or_else(|| s.pid.and_then(|p| pids.get(&p).copied()));
            let (mut rx, mut tx) = (0.0, 0.0);
            if s.tcp {
                if let Some(dt) = dt {
                    // socket new since the last sample (or counters reset to zero): count everything
                    let (pr, pt) = self.prev.get(&(s.local.clone(), s.peer.clone())).copied().unwrap_or((0, 0));
                    let d = |c: u64, p: u64| if c >= p { c - p } else { c };
                    rx = d(s.rx, pr) as f64 / dt;
                    tx = d(s.tx, pt) as f64 / dt;
                }
                prev.insert((s.local, s.peer), (s.rx, s.tx));
            }
            let a = accs.entry(gi).or_default();
            a.recv += rx;
            a.send += tx;
            a.conns += s.estab as u32;
            let r = a.remotes.entry(remote).or_default();
            r.0 += rx + tx;
            r.1 += s.rx + s.tx;
        }
        self.prev = prev;

        let mut out: Vec<NetApp> = accs
            .into_iter()
            .filter(|(_, a)| a.conns > 0 || a.recv + a.send > 0.0)
            .map(|(gi, a)| {
                let (key, label, icon, section) = match gi.map(|i| &snap.groups[i]) {
                    Some(g) => (g.key.clone(), g.label.clone(), g.icon.clone(), g.section),
                    None => ("other".into(), crate::i18n::t("network.others"), None, Section::Sys),
                };
                let mut r: Vec<(String, (f64, u64))> = a.remotes.into_iter().collect();
                r.sort_by(|x, y| y.1 .0.total_cmp(&x.1 .0).then(y.1 .1.cmp(&x.1 .1)).then_with(|| x.0.cmp(&y.0)));
                let remotes = r.into_iter().take(3).map(|x| x.0).collect();
                NetApp { key, label, icon, section, recv: a.recv, send: a.send, conns: a.conns, remotes }
            })
            .collect();
        out.sort_by(|a, b| (b.recv + b.send).total_cmp(&(a.recv + a.send)).then(b.conns.cmp(&a.conns)).then_with(|| a.label.cmp(&b.label)));
        out
    }
}

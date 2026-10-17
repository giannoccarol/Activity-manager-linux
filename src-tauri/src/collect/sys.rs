//! System resources: CPU, memory, disks, network.
use crate::model::{Cpu, Disk, Mem, Net};
use crate::util::{read, read_raw, run, which};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Default)]
struct CpuT {
    busy: u64,
    kernel: u64,
    iowait: u64,
    total: u64,
}

#[derive(Clone, Copy)]
struct DiskT {
    rd_sec: u64,
    wr_sec: u64,
    ios: u64,
    ms_rw: u64,
    io_ms: u64,
}

struct DiskStatic {
    model: String,
    kind: String,
    capacity: u64,
    system: bool,
}

pub struct Sys {
    prev_cpu: Vec<CpuT>,
    prev_disk: HashMap<String, DiskT>,
    prev_net: HashMap<String, (u64, u64)>,
    prev_swap: Option<(u64, u64, Instant)>,
    disk_static: HashMap<String, DiskStatic>,
    disk_static_t: Option<Instant>,
    wifi: HashMap<String, (String, f64)>,
    wifi_t: Option<Instant>,
    temp_path: Option<String>,
    pub model: String,
    page: u64,
}

fn parse_cpu_line(l: &str) -> CpuT {
    let v: Vec<u64> = l.split_whitespace().skip(1).filter_map(|x| x.parse().ok()).collect();
    let g = |i: usize| v.get(i).copied().unwrap_or(0);
    // user nice system idle iowait irq softirq steal
    let idle = g(3);
    let iowait = g(4);
    let total: u64 = v.iter().take(8).sum();
    CpuT { busy: total - idle - iowait, kernel: g(2) + g(5) + g(6), iowait, total }
}

fn find_temp() -> Option<String> {
    let rd = std::fs::read_dir("/sys/class/hwmon").ok()?;
    let mut fallback = None;
    for e in rd.flatten() {
        let p = e.path();
        let name = read(p.join("name")).unwrap_or_default();
        if !["coretemp", "k10temp", "zenpower", "cpu_thermal"].contains(&name.as_str()) {
            continue;
        }
        for i in 1..32 {
            let input = p.join(format!("temp{i}_input"));
            if !input.exists() {
                continue;
            }
            let label = read(p.join(format!("temp{i}_label"))).unwrap_or_default();
            if label.starts_with("Package") || label == "Tctl" || label == "Tdie" {
                return Some(input.to_string_lossy().into_owned());
            }
            if fallback.is_none() {
                fallback = Some(input.to_string_lossy().into_owned());
            }
        }
    }
    fallback
}

fn base_disks(name: &str, depth: u32) -> HashSet<String> {
    let p = format!("/sys/class/block/{name}");
    let mut out = HashSet::new();
    if depth > 8 || !std::path::Path::new(&p).exists() {
        return out;
    }
    if let Ok(rd) = std::fs::read_dir(format!("{p}/slaves")) {
        let sl: Vec<String> = rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        if !sl.is_empty() {
            for s in sl {
                out.extend(base_disks(&s, depth + 1));
            }
            return out;
        }
    }
    if std::path::Path::new(&format!("{p}/partition")).exists() {
        if let Ok(real) = std::fs::canonicalize(&p) {
            if let Some(parent) = real.parent().and_then(|x| x.file_name()) {
                out.insert(parent.to_string_lossy().into_owned());
                return out;
            }
        }
    }
    out.insert(name.to_string());
    out
}

impl Sys {
    pub fn new() -> Self {
        let model = read_raw("/proc/cpuinfo")
            .and_then(|t| t.lines().find_map(|l| l.strip_prefix("model name").map(|r| r.trim_start_matches([' ', '\t', ':']).to_string())))
            .unwrap_or_else(|| "CPU".into())
            .replace("(R)", "")
            .replace("(TM)", "")
            .replace("  ", " ");
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
        Sys {
            prev_cpu: vec![],
            prev_disk: HashMap::new(),
            prev_net: HashMap::new(),
            prev_swap: None,
            disk_static: HashMap::new(),
            disk_static_t: None,
            wifi: HashMap::new(),
            wifi_t: None,
            temp_path: find_temp(),
            model,
            page: if page > 0 { page } else { 4096 },
        }
    }

    pub fn cpu(&mut self) -> Cpu {
        let txt = read_raw("/proc/stat").unwrap_or_default();
        let cur: Vec<CpuT> = txt.lines().filter(|l| l.starts_with("cpu")).map(parse_cpu_line).collect();
        let mut per = vec![];
        let (mut total, mut kernel, mut iowait) = (0.0, 0.0, 0.0);
        if cur.len() == self.prev_cpu.len() {
            for (i, (c, p)) in cur.iter().zip(self.prev_cpu.iter()).enumerate() {
                let dt = c.total.saturating_sub(p.total).max(1) as f64;
                let busy = c.busy.saturating_sub(p.busy) as f64 / dt * 100.0;
                if i == 0 {
                    total = busy.clamp(0.0, 100.0);
                    kernel = (c.kernel.saturating_sub(p.kernel) as f64 / dt * 100.0).clamp(0.0, 100.0);
                    iowait = (c.iowait.saturating_sub(p.iowait) as f64 / dt * 100.0).clamp(0.0, 100.0);
                } else {
                    per.push(busy.clamp(0.0, 100.0));
                }
            }
        } else {
            per = vec![0.0; cur.len().saturating_sub(1)];
        }
        self.prev_cpu = cur;
        let mut freqs = vec![];
        for i in 0..per.len() {
            if let Some(v) = read(format!("/sys/devices/system/cpu/cpu{i}/cpufreq/scaling_cur_freq")).and_then(|v| v.parse::<f64>().ok()) {
                freqs.push(v / 1000.0);
            }
        }
        let freq = if freqs.is_empty() { 0.0 } else { freqs.iter().sum::<f64>() / freqs.len() as f64 };
        let temp = self.temp_path.as_ref().and_then(|p| read(p)).and_then(|v| v.parse::<f64>().ok()).map(|v| v / 1000.0);
        Cpu { total, kernel, iowait, per_core: per, freq, temp, model: self.model.clone() }
    }

    pub fn mem(&mut self) -> Mem {
        let mut mi: HashMap<&str, u64> = HashMap::new();
        let txt = read_raw("/proc/meminfo").unwrap_or_default();
        for l in txt.lines() {
            if let Some((k, v)) = l.split_once(':') {
                let n: u64 = v.split_whitespace().next().and_then(|x| x.parse().ok()).unwrap_or(0);
                mi.insert(k, n * 1024);
            }
        }
        let g = |k: &str| mi.get(k).copied().unwrap_or(0);
        let total = g("MemTotal");
        let available = g("MemAvailable");
        let used = total.saturating_sub(available);
        let vm = read_raw("/proc/vmstat").unwrap_or_default();
        let (mut pin, mut pout) = (0u64, 0u64);
        for l in vm.lines() {
            if let Some(v) = l.strip_prefix("pswpin ") {
                pin = v.parse().unwrap_or(0);
            } else if let Some(v) = l.strip_prefix("pswpout ") {
                pout = v.parse().unwrap_or(0);
            }
        }
        let now = Instant::now();
        let (mut sin, mut sout) = (0.0, 0.0);
        if let Some((pi, po, t)) = self.prev_swap {
            let el = now.duration_since(t).as_secs_f64().max(0.001);
            sin = pin.saturating_sub(pi) as f64 * self.page as f64 / el;
            sout = pout.saturating_sub(po) as f64 * self.page as f64 / el;
        }
        self.prev_swap = Some((pin, pout, now));
        Mem {
            total,
            used,
            available,
            pct: if total > 0 { used as f64 / total as f64 * 100.0 } else { 0.0 },
            swap_used: g("SwapTotal").saturating_sub(g("SwapFree")),
            swap_total: g("SwapTotal"),
            cached: g("Cached") + g("SReclaimable") + g("Buffers"),
            swap_in: sin,
            swap_out: sout,
        }
    }

    fn refresh_disk_static(&mut self) {
        let mut disks = HashMap::new();
        let Ok(rd) = std::fs::read_dir("/sys/block") else { return };
        for e in rd.flatten() {
            let d = e.file_name().to_string_lossy().into_owned();
            if ["loop", "ram", "zram", "dm-", "md", "sr", "fd"].iter().any(|p| d.starts_with(p)) {
                continue;
            }
            let base = format!("/sys/block/{d}");
            if !std::path::Path::new(&format!("{base}/device")).exists() {
                continue;
            }
            let model = read(format!("{base}/device/model")).unwrap_or_else(|| d.clone());
            let rot = read(format!("{base}/queue/rotational")).as_deref() == Some("1");
            let usb = std::fs::canonicalize(&base).map(|p| p.to_string_lossy().contains("/usb")).unwrap_or(false);
            let kind = if usb {
                "Rimovibile (USB)"
            } else if d.starts_with("nvme") {
                "SSD NVMe"
            } else if rot {
                "Disco rigido"
            } else {
                "SSD"
            };
            let capacity = read(format!("{base}/size")).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0) * 512;
            disks.insert(d, DiskStatic { model, kind: kind.into(), capacity, system: false });
        }
        if let Some(m) = read_raw("/proc/mounts") {
            for l in m.lines() {
                let f: Vec<&str> = l.split_whitespace().collect();
                if f.len() > 1 && f[1] == "/" && f[0].starts_with("/dev/") {
                    let dev = std::fs::canonicalize(f[0]).map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| f[0].into());
                    let name = dev.trim_start_matches("/dev/").to_string();
                    for b in base_disks(&name, 0) {
                        if let Some(ds) = disks.get_mut(&b) {
                            ds.system = true;
                        }
                    }
                }
            }
        }
        self.disk_static = disks;
        self.disk_static_t = Some(Instant::now());
    }

    pub fn disks(&mut self, dt: Option<f64>) -> Vec<Disk> {
        if self.disk_static_t.map(|t| t.elapsed() > Duration::from_secs(15)).unwrap_or(true) {
            self.refresh_disk_static();
        }
        let txt = read_raw("/proc/diskstats").unwrap_or_default();
        let mut out = vec![];
        for l in txt.lines() {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.len() < 14 {
                continue;
            }
            let name = f[2];
            let Some(st) = self.disk_static.get(name) else { continue };
            let n = |i: usize| f[i].parse::<u64>().unwrap_or(0);
            let cur = DiskT { rd_sec: n(5), wr_sec: n(9), ios: n(3) + n(7), ms_rw: n(6) + n(10), io_ms: n(12) };
            let mut d = Disk {
                name: name.to_string(),
                model: st.model.clone(),
                kind: st.kind.clone(),
                active: 0.0,
                read: 0.0,
                write: 0.0,
                resp: 0.0,
                capacity: st.capacity,
                system: st.system,
            };
            if let (Some(p), Some(dt)) = (self.prev_disk.get(name), dt) {
                if dt > 0.0 {
                    d.active = (cur.io_ms.saturating_sub(p.io_ms) as f64 / (dt * 1000.0) * 100.0).clamp(0.0, 100.0);
                    d.read = cur.rd_sec.saturating_sub(p.rd_sec) as f64 * 512.0 / dt;
                    d.write = cur.wr_sec.saturating_sub(p.wr_sec) as f64 * 512.0 / dt;
                    let ops = cur.ios.saturating_sub(p.ios);
                    d.resp = if ops > 0 { cur.ms_rw.saturating_sub(p.ms_rw) as f64 / ops as f64 } else { 0.0 };
                }
            }
            self.prev_disk.insert(name.to_string(), cur);
            out.push(d);
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    fn refresh_wifi(&mut self) {
        self.wifi_t = Some(Instant::now());
        if !which("nmcli") {
            return;
        }
        let out = run("nmcli", &["-t", "-f", "IN-USE,SSID,RATE,DEVICE", "dev", "wifi", "list", "--rescan", "no"], Duration::from_secs(3));
        let mut m = HashMap::new();
        for l in out.lines().filter(|l| l.starts_with('*')) {
            let f: Vec<&str> = l.split(':').collect();
            if f.len() >= 4 {
                let rate = f[2].split_whitespace().next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0) * 1e6 / 8.0;
                m.insert(f[3].to_string(), (f[1].to_string(), rate));
            }
        }
        self.wifi = m;
    }

    pub fn nets(&mut self, dt: Option<f64>) -> Vec<Net> {
        if self.wifi_t.map(|t| t.elapsed() > Duration::from_secs(15)).unwrap_or(true) {
            self.refresh_wifi();
        }
        let txt = read_raw("/proc/net/dev").unwrap_or_default();
        let mut out = vec![];
        for l in txt.lines().skip(2) {
            let Some((name, rest)) = l.split_once(':') else { continue };
            let name = name.trim();
            if name == "lo" {
                continue;
            }
            let base = format!("/sys/class/net/{name}");
            let phys = std::path::Path::new(&format!("{base}/device")).exists();
            let wifi = std::path::Path::new(&format!("{base}/wireless")).exists() || std::path::Path::new(&format!("{base}/phy80211")).exists();
            let oper = read(format!("{base}/operstate")).unwrap_or_default();
            let vpn = ["wg", "tun", "tailscale", "ppp", "tap", "proton", "nordlynx", "zt", "mullvad"].iter().any(|p| name.starts_with(p));
            let up = oper == "up" || oper == "unknown";
            if !(phys || (vpn && up)) {
                continue;
            }
            let v: Vec<u64> = rest.split_whitespace().filter_map(|x| x.parse().ok()).collect();
            if v.len() < 9 {
                continue;
            }
            let (rx, tx) = (v[0], v[8]);
            let (mut recv, mut send) = (0.0, 0.0);
            if let (Some((prx, ptx)), Some(dt)) = (self.prev_net.get(name), dt) {
                if dt > 0.0 {
                    recv = rx.saturating_sub(*prx) as f64 / dt;
                    send = tx.saturating_sub(*ptx) as f64 / dt;
                }
            }
            self.prev_net.insert(name.to_string(), (rx, tx));
            let w = self.wifi.get(name);
            let speed = read(format!("{base}/speed")).and_then(|v| v.parse::<f64>().ok()).filter(|v| *v > 0.0).unwrap_or(0.0) * 1e6 / 8.0;
            out.push(Net {
                name: name.to_string(),
                kind: if wifi { "Wi-Fi" } else if vpn { "VPN" } else { "Ethernet" }.into(),
                connected: oper == "up" || (vpn && up),
                ssid: w.map(|x| x.0.clone()),
                send,
                recv,
                link: w.map(|x| x.1).filter(|v| *v > 0.0).unwrap_or(speed),
            });
        }
        out.sort_by_key(|n| (!n.connected, n.kind == "VPN", n.name.clone()));
        out
    }
}

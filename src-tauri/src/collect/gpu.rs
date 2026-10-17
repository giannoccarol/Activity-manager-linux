//! GPU: utilizzo per motore e per processo dalle statistiche DRM (fdinfo) del kernel;
//! NVIDIA interrogata solo se già attiva e in uso (non viene mai risvegliata).
use super::procs::{Key, Sample};
use crate::model::Gpu;
use crate::util::{read, read_raw, run, which};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct Dev {
    pdev: String,
    vendor: String,
    name: String,
    card: Option<String>,
    sysdir: String,
    integrated: bool,
}

#[derive(Default, Clone)]
struct NvData {
    util: f64,
    mem_used: u64,
    mem_total: u64,
    temp: Option<f64>,
    at: Option<Instant>,
}

struct ClientPrev {
    t: Instant,
    ns: HashMap<String, u64>,
    cyc: HashMap<String, (u64, u64)>,
}

pub struct Gpus {
    devs: Vec<Dev>,
    my_uid: u32,
    my_pid: i32,
    drm_fds: HashMap<Key, (Vec<String>, bool, u64)>,
    prev: HashMap<(String, String), ClientPrev>,
    nv: Arc<Mutex<NvData>>,
    nv_busy: Arc<Mutex<bool>>,
    tick: u64,
}

fn engine_label(e: &str) -> &str {
    match e {
        "render" | "rcs" | "gfx" | "graphics" => "3D",
        "copy" | "bcs" | "sdma" => "Copia",
        "video" | "vcs" | "dec" => "Video",
        "enc" => "Codifica video",
        "video-enhance" | "vecs" => "Elaborazione video",
        "compute" | "ccs" => "Calcolo",
        other => other,
    }
}

fn pci_name(vendor: &str, device: &str) -> String {
    let v = vendor.trim_start_matches("0x").to_lowercase();
    let d = device.trim_start_matches("0x").to_lowercase();
    let short = match v.as_str() {
        "10de" => "NVIDIA",
        "8086" => "Intel",
        "1002" => "AMD",
        _ => "GPU",
    };
    for path in ["/usr/share/hwdata/pci.ids", "/usr/share/misc/pci.ids"] {
        let Some(txt) = read_raw(path) else { continue };
        let mut in_vendor = false;
        for l in txt.lines() {
            if l.starts_with('#') || l.is_empty() {
                continue;
            }
            if !l.starts_with('\t') {
                in_vendor = l.to_lowercase().starts_with(&v);
                continue;
            }
            if in_vendor && !l.starts_with("\t\t") {
                let t = l.trim();
                if t.to_lowercase().starts_with(&d) {
                    let desc = t[d.len()..].trim();
                    let name = desc.find('[').and_then(|a| desc.rfind(']').map(|b| desc[a + 1..b].to_string())).unwrap_or_else(|| desc.to_string());
                    return if name.to_lowercase().contains(&short.to_lowercase()) { name } else { format!("{short} {name}") };
                }
            }
        }
    }
    format!("{short} GPU")
}

impl Gpus {
    pub fn new() -> Self {
        let mut cards = HashMap::new();
        if let Ok(rd) = std::fs::read_dir("/sys/class/drm") {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if n.starts_with("card") && n[4..].chars().all(|c| c.is_ascii_digit()) {
                    if let Ok(dev) = std::fs::canonicalize(e.path().join("device")) {
                        cards.insert(dev, e.path().to_string_lossy().into_owned());
                    }
                }
            }
        }
        let mut devs = vec![];
        let mut entries: Vec<_> = std::fs::read_dir("/sys/bus/pci/devices").map(|r| r.flatten().map(|e| e.path()).collect()).unwrap_or_default();
        entries.sort();
        for p in entries {
            let class = read(p.join("class")).unwrap_or_default();
            if !class.starts_with("0x03") {
                continue;
            }
            let vendor = read(p.join("vendor")).unwrap_or_default();
            let pdev = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let real = std::fs::canonicalize(&p).unwrap_or(p.clone());
            devs.push(Dev {
                vendor: match vendor.as_str() {
                    "0x8086" => "intel",
                    "0x10de" => "nvidia",
                    "0x1002" => "amd",
                    _ => "other",
                }
                .into(),
                name: pci_name(&vendor, &read(p.join("device")).unwrap_or_default()),
                card: cards.get(&real).cloned(),
                sysdir: p.to_string_lossy().into_owned(),
                integrated: pdev.starts_with("0000:00:"),
                pdev,
            });
        }
        Gpus {
            devs,
            my_uid: unsafe { libc::getuid() },
            my_pid: std::process::id() as i32,
            drm_fds: HashMap::new(),
            prev: HashMap::new(),
            nv: Arc::new(Mutex::new(NvData::default())),
            nv_busy: Arc::new(Mutex::new(false)),
            tick: 0,
        }
    }

    fn fds(&mut self, key: Key, pid: i32) -> (Vec<String>, bool) {
        if let Some((f, nv, t)) = self.drm_fds.get(&key) {
            if self.tick.saturating_sub(*t) < 10 {
                return (f.clone(), *nv);
            }
        }
        let mut fds = vec![];
        let mut nv = false;
        if let Ok(rd) = std::fs::read_dir(format!("/proc/{pid}/fd")) {
            for e in rd.flatten() {
                if let Ok(l) = std::fs::read_link(e.path()) {
                    let l = l.to_string_lossy();
                    if l.starts_with("/dev/dri/") {
                        fds.push(e.file_name().to_string_lossy().into_owned());
                    } else if l.starts_with("/dev/nvidia") && l.as_bytes().get(11).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                        nv = true;
                    }
                }
            }
        }
        // sfalsa le riscansioni tra i processi
        let stagger = if self.drm_fds.contains_key(&key) { 0 } else { (pid as u64) % 10 };
        self.drm_fds.insert(key, (fds.clone(), nv, self.tick.saturating_sub(stagger)));
        (fds, nv)
    }

    /// Calcola l'uso GPU per processo e restituisce lo stato delle GPU.
    pub fn collect(&mut self, procs: &mut [Sample], owners: &HashMap<Key, (u32, bool, String)>) -> Vec<Gpu> {
        self.tick += 1;
        let now = Instant::now();
        let mut engines: HashMap<String, BTreeMap<String, f64>> = HashMap::new();
        let mut shared: HashMap<String, u64> = HashMap::new();
        let mut seen_clients = HashSet::new();
        let mut nv_clients = 0usize;
        let idx: HashMap<String, usize> = self.devs.iter().enumerate().map(|(i, d)| (d.pdev.clone(), i)).collect();
        for p in procs.iter_mut() {
            let Some((uid, kernel, name)) = owners.get(&p.key) else { continue };
            if *kernel || !(*uid == self.my_uid || self.my_uid == 0) {
                continue;
            }
            let (fds, nv) = self.fds(p.key, p.pid);
            if nv && name != "nvidia-smi" && p.ppid != self.my_pid {
                nv_clients += 1;
            }
            let (mut best, mut best_eng, mut pmem) = (0.0f64, String::new(), 0u64);
            for fd in fds {
                let Some(txt) = read_raw(format!("/proc/{}/fdinfo/{fd}", p.pid)) else { continue };
                let (mut pdev, mut cid) = (None, None);
                let (mut ns, mut cyc, mut tot, mut cap) = (HashMap::new(), HashMap::new(), HashMap::new(), HashMap::new());
                let mut mem = 0u64;
                for l in txt.lines() {
                    let Some((k, v)) = l.split_once(':') else { continue };
                    let v = v.trim();
                    let num = || v.split_whitespace().next().and_then(|x| x.parse::<u64>().ok()).unwrap_or(0);
                    if k == "drm-pdev" {
                        pdev = Some(v.to_string());
                    } else if k == "drm-client-id" {
                        cid = Some(v.to_string());
                    } else if let Some(e) = k.strip_prefix("drm-engine-capacity-") {
                        cap.insert(e.to_string(), num().max(1));
                    } else if let Some(e) = k.strip_prefix("drm-engine-") {
                        ns.insert(e.to_string(), num());
                    } else if let Some(e) = k.strip_prefix("drm-cycles-") {
                        cyc.insert(e.to_string(), num());
                    } else if let Some(e) = k.strip_prefix("drm-total-cycles-") {
                        tot.insert(e.to_string(), num());
                    } else if k.starts_with("drm-resident-") || k.starts_with("drm-memory-") {
                        let mult = if v.ends_with("MiB") { 1 << 20 } else if v.ends_with("GiB") { 1 << 30 } else if v.ends_with("KiB") { 1024 } else { 1 };
                        mem += num() * mult;
                    }
                }
                let (Some(pdev), Some(cid)) = (pdev, cid) else { continue };
                let ck = (pdev.clone(), cid);
                if !seen_clients.insert(ck.clone()) {
                    continue;
                }
                *shared.entry(pdev.clone()).or_default() += mem;
                pmem += mem;
                let mut cycles = HashMap::new();
                for (e, c) in &cyc {
                    if let Some(t) = tot.get(e) {
                        cycles.insert(e.clone(), (*c, *t));
                    }
                }
                let prev = self.prev.insert(ck, ClientPrev { t: now, ns: ns.clone(), cyc: cycles.clone() });
                let Some(prev) = prev else { continue };
                let el = now.duration_since(prev.t).as_secs_f64();
                if el <= 0.0 {
                    continue;
                }
                let gidx = idx.get(&pdev).copied().unwrap_or(0);
                let ent = engines.entry(pdev.clone()).or_default();
                for (e, v) in &ns {
                    if let Some(pv) = prev.ns.get(e) {
                        let u = (v.saturating_sub(*pv) as f64 / (el * 1e9) / *cap.get(e).unwrap_or(&1) as f64 * 100.0).max(0.0);
                        *ent.entry(engine_label(e).to_string()).or_default() += u;
                        if u > best {
                            best = u;
                            best_eng = format!("{} (GPU {gidx})", engine_label(e));
                        }
                    }
                }
                for (e, (c, t)) in &cycles {
                    if let Some((pc, pt)) = prev.cyc.get(e) {
                        if t > pt {
                            let u = (c.saturating_sub(*pc) as f64 / (t - pt) as f64 * 100.0).max(0.0);
                            *ent.entry(engine_label(e).to_string()).or_default() += u;
                            if u > best {
                                best = u;
                                best_eng = format!("{} (GPU {gidx})", engine_label(e));
                            }
                        }
                    }
                }
            }
            p.gpu = best.min(100.0);
            if best >= 0.05 {
                p.gpu_engine = if self.devs.len() > 1 { best_eng } else { best_eng.split(" (").next().unwrap_or("").to_string() };
            }
            p.gpu_mem = pmem;
        }
        if self.tick % 20 == 0 {
            self.prev.retain(|_, v| now.duration_since(v.t) < Duration::from_secs(30));
            let live: HashSet<Key> = procs.iter().map(|p| p.key).collect();
            self.drm_fds.retain(|k, _| live.contains(k));
        }
        let mut out = vec![];
        for (i, d) in self.devs.iter().enumerate() {
            let mut eng: BTreeMap<String, f64> = engines.remove(&d.pdev).unwrap_or_default().into_iter().map(|(k, v)| (k, v.min(100.0))).collect();
            let mut util = eng.values().cloned().fold(0.0, f64::max);
            let mut g = Gpu {
                idx: i,
                name: d.name.clone(),
                vendor: d.vendor.clone(),
                util,
                engines: BTreeMap::new(),
                mem_used: shared.get(&d.pdev).copied().unwrap_or(0),
                mem_total: None,
                temp: None,
                suspended: false,
            };
            if d.vendor == "amd" {
                if let Some(c) = &d.card {
                    if let Some(b) = read(format!("{c}/device/gpu_busy_percent")).and_then(|v| v.parse::<f64>().ok()) {
                        util = util.max(b);
                        eng.entry("3D".into()).or_insert(b);
                    }
                    if let (Some(u), Some(t)) = (read(format!("{c}/device/mem_info_vram_used")), read(format!("{c}/device/mem_info_vram_total"))) {
                        g.mem_used = u.parse().unwrap_or(0);
                        g.mem_total = t.parse().ok();
                    }
                }
            }
            if d.vendor == "nvidia" {
                let status = read(format!("{}/power/runtime_status", d.sysdir)).unwrap_or_default();
                g.suspended = status == "suspended";
                if status == "active" && nv_clients > 0 {
                    self.poll_nvidia();
                }
                let nd = self.nv.lock().map(|x| x.clone()).unwrap_or_default();
                if status == "active" && nd.at.map(|t| t.elapsed() < Duration::from_secs(6)).unwrap_or(false) {
                    util = nd.util;
                    eng.insert("3D".into(), nd.util);
                    g.mem_used = nd.mem_used;
                    g.temp = nd.temp;
                }
                if nd.mem_total > 0 {
                    g.mem_total = Some(nd.mem_total);
                }
            }
            if d.integrated && g.mem_total.is_none() {
                g.mem_total = read_raw("/proc/meminfo")
                    .and_then(|t| t.lines().next().and_then(|l| l.split_whitespace().nth(1)).and_then(|v| v.parse::<u64>().ok()))
                    .map(|kb| kb * 1024 / 2);
            }
            g.util = util;
            g.engines = eng;
            out.push(g);
        }
        out
    }

    fn poll_nvidia(&self) {
        if !which("nvidia-smi") {
            return;
        }
        {
            let mut busy = self.nv_busy.lock().unwrap();
            let fresh = self.nv.lock().map(|x| x.at.map(|t| t.elapsed() < Duration::from_secs(2)).unwrap_or(false)).unwrap_or(false);
            if *busy || fresh {
                return;
            }
            *busy = true;
        }
        let nv = self.nv.clone();
        let busy = self.nv_busy.clone();
        std::thread::spawn(move || {
            let out = run(
                "nvidia-smi",
                &["--query-gpu=utilization.gpu,memory.used,memory.total,temperature.gpu", "--format=csv,noheader,nounits"],
                Duration::from_secs(6),
            );
            if let Some(l) = out.lines().next() {
                let v: Vec<f64> = l.split(',').map(|x| x.trim().parse().unwrap_or(0.0)).collect();
                if v.len() >= 4 {
                    if let Ok(mut d) = nv.lock() {
                        *d = NvData { util: v[0], mem_used: (v[1] * 1048576.0) as u64, mem_total: (v[2] * 1048576.0) as u64, temp: Some(v[3]), at: Some(Instant::now()) };
                    }
                }
            }
            *busy.lock().unwrap() = false;
        });
    }
}

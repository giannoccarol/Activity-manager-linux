//! Hardware sensors device by device, like HWiNFO: hwmon (temperatures, fans, voltages, currents,
//! power), ACPI thermal zones, per-core clocks, GPUs and battery, with min/max/average since start.
//! Sampled by the collector at every update, so the statistics cover the whole session. Current
//! temperatures are smoothed over a few seconds (see util::Smooth); min, max and average use the raw
//! readings, so boost peaks stay visible as the maximum.
use crate::i18n::{t, tf};
use crate::model::Snapshot;
use crate::util::{read, read_raw, Smooth};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
    pub id: String,
    pub label: String,
    pub sub: Option<String>,
    /// temp, fan, volt, current, power, clock, percent, count, energy, memory
    pub kind: &'static str,
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub warn: Option<f64>,
    pub crit: Option<f64>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    /// cpu, gpu, disk, memory, network, board, battery
    pub kind: &'static str,
    pub note: Option<String>,
    pub readings: Vec<Reading>,
}

struct Raw {
    id: String,
    label: String,
    sub: Option<String>,
    kind: &'static str,
    value: f64,
    warn: Option<f64>,
    crit: Option<f64>,
    order: u32,
}

#[derive(Clone, Copy)]
struct Stat {
    min: f64,
    max: f64,
    sum: f64,
    n: u64,
}

/// Physical cores: (package, core_id) → logical CPUs and type (P/E on hybrid CPUs).
struct Core {
    label: String,
    cpus: Vec<usize>,
    order: u32,
}

pub struct Sensors {
    stats: HashMap<String, Stat>,
    smooth: HashMap<String, Smooth>,
    cores: HashMap<(u32, u32), Core>,
    lang: &'static str,
}

fn num(path: &Path) -> Option<f64> {
    read(path).and_then(|v| v.trim().parse::<f64>().ok())
}

fn cpu_list(s: &str) -> Vec<usize> {
    let mut out = vec![];
    for part in s.trim().split(',').filter(|p| !p.is_empty()) {
        match part.split_once('-') {
            Some((a, b)) => {
                if let (Ok(a), Ok(b)) = (a.parse::<usize>(), b.parse::<usize>()) {
                    out.extend(a..=b);
                }
            }
            None => out.extend(part.parse::<usize>().ok()),
        }
    }
    out
}

/// Device description from pci.ids (vendor and device ids in hex).
fn pci_device_name(vendor: &str, device: &str) -> Option<String> {
    let v = vendor.trim_start_matches("0x").to_lowercase();
    let d = device.trim_start_matches("0x").to_lowercase();
    for path in ["/usr/share/hwdata/pci.ids", "/usr/share/misc/pci.ids"] {
        let Some(txt) = read_raw(path) else { continue };
        let mut vname = String::new();
        let mut in_vendor = false;
        for l in txt.lines() {
            if l.starts_with('#') || l.is_empty() {
                continue;
            }
            if !l.starts_with('\t') {
                in_vendor = l.to_lowercase().starts_with(&v);
                if in_vendor {
                    vname = l[v.len()..].trim().split_whitespace().next().unwrap_or("").to_string();
                }
                continue;
            }
            if in_vendor && !l.starts_with("\t\t") && l.trim().to_lowercase().starts_with(&d) {
                let desc = l.trim()[d.len()..].trim();
                return Some(if desc.to_lowercase().starts_with(&vname.to_lowercase()) { desc.to_string() } else { format!("{vname} {desc}") });
            }
        }
    }
    None
}

/// Name of a PCI device from its sysfs directory (uevent PCI_ID).
fn pci_name_of(dev: &Path) -> Option<String> {
    let ue = read_raw(dev.join("uevent"))?;
    let id = ue.lines().find_map(|l| l.strip_prefix("PCI_ID="))?;
    let (v, d) = id.split_once(':')?;
    pci_device_name(v, d)
}

fn temp_limits(crit: Option<f64>) -> (Option<f64>, Option<f64>) {
    match crit {
        Some(c) if c > 30.0 && c < 200.0 => (Some(c - 15.0), Some(c - 3.0)),
        _ => (Some(80.0), Some(90.0)),
    }
}

impl Sensors {
    pub fn new() -> Self {
        let mut s = Sensors { stats: HashMap::new(), smooth: HashMap::new(), cores: HashMap::new(), lang: crate::i18n::lang() };
        s.build_cores();
        s
    }

    fn build_cores(&mut self) {
        self.cores.clear();
        let p: Vec<usize> = read("/sys/devices/cpu_core/cpus").map(|v| cpu_list(&v)).unwrap_or_default();
        let e: Vec<usize> = read("/sys/devices/cpu_atom/cpus").map(|v| cpu_list(&v)).unwrap_or_default();
        let hybrid = !p.is_empty() && !e.is_empty();
        let mut map: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
        if let Ok(rd) = std::fs::read_dir("/sys/devices/system/cpu") {
            for ent in rd.flatten() {
                let name = ent.file_name().to_string_lossy().into_owned();
                let Some(n) = name.strip_prefix("cpu").and_then(|v| v.parse::<usize>().ok()) else { continue };
                let topo = ent.path().join("topology");
                let (Some(core), Some(pkg)) = (num(&topo.join("core_id")), num(&topo.join("physical_package_id"))) else { continue };
                map.entry((pkg as u32, core as u32)).or_default().push(n);
            }
        }
        let multi_pkg = map.keys().map(|k| k.0).collect::<std::collections::HashSet<_>>().len() > 1;
        let (mut pi, mut ei, mut ci) = (0u32, 0u32, 0u32);
        for ((pkg, core), mut cpus) in map {
            cpus.sort_unstable();
            let is_p = hybrid && cpus.iter().any(|c| p.contains(c));
            let is_e = hybrid && !is_p;
            let (label, order) = if is_p {
                pi += 1;
                (tf("sensors.pcore", &[("n", (pi - 1).to_string())]), pi)
            } else if is_e {
                ei += 1;
                (tf("sensors.ecore", &[("n", (ei - 1).to_string())]), 100 + ei)
            } else {
                ci += 1;
                (tf("sensors.core", &[("n", (ci - 1).to_string())]), ci)
            };
            let label = if multi_pkg { format!("{label} ({pkg})") } else { label };
            self.cores.insert((pkg, core), Core { label, cpus, order });
        }
    }

    pub fn reset(&mut self) {
        self.stats.clear();
    }

    fn cpus_sub(cpus: &[usize]) -> String {
        tf("sensors.logical", &[("list", cpus.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", "))])
    }

    /// Reads every sensor and returns the devices with their readings and statistics.
    pub fn sample(&mut self, snap: Option<&Snapshot>) -> Vec<Device> {
        if self.lang != crate::i18n::lang() {
            self.lang = crate::i18n::lang();
            self.build_cores();
        }
        // device id → (name, kind, note, raw readings)
        let mut devs: BTreeMap<(u32, String), (String, &'static str, Option<String>, Vec<Raw>)> = BTreeMap::new();
        let cpu_name = snap.map(|s| s.cpu.model.clone()).filter(|m| !m.is_empty()).unwrap_or_else(|| t("sensors.cpu"));
        let mut has_coretemp = false;
        let mut seen_zones: Vec<String> = vec![];
        let mut dimm = 0u32;

        let mut hwmons: Vec<PathBuf> = std::fs::read_dir("/sys/class/hwmon").map(|rd| rd.flatten().map(|e| e.path()).collect()).unwrap_or_default();
        hwmons.sort_by_key(|p| p.file_name().and_then(|n| n.to_str()).and_then(|n| n.trim_start_matches("hwmon").parse::<u32>().ok()).unwrap_or(0));
        for h in hwmons {
            let name = read(h.join("name")).unwrap_or_default();
            let dev = std::fs::canonicalize(h.join("device")).unwrap_or_default();
            let devs_str = dev.to_string_lossy().into_owned();
            if devs_str.contains("/thermal/thermal_zone") {
                if let Some(z) = dev.file_name() {
                    seen_zones.push(z.to_string_lossy().into_owned());
                }
            }
            // which device and how to label the readings of this hwmon
            let (dkey, dname, dkind): ((u32, String), String, &'static str) = match name.as_str() {
                "coretemp" | "k10temp" | "zenpower" => {
                    has_coretemp = true;
                    ((0, "cpu".into()), cpu_name.clone(), "cpu")
                }
                "nvme" => {
                    let ctrl = devs_str.rsplit('/').next().unwrap_or("nvme").to_string();
                    let model = read(format!("/sys/class/nvme/{ctrl}/model")).map(|m| m.trim().replace('_', " ")).filter(|m| !m.is_empty());
                    ((30, ctrl.clone()), model.unwrap_or(ctrl), "disk")
                }
                "spd5118" | "jc42" => ((20, "memory".into()), if name == "spd5118" { tf("sensors.memoryKind", &[("kind", "DDR5".into())]) } else { t("sensors.memory") }, "memory"),
                n if n.starts_with("iwlwifi") || n.starts_with("mt79") || n.starts_with("ath1") || n.starts_with("rtw") => {
                    let wifi = std::fs::read_dir("/sys/class/net").ok().and_then(|rd| rd.flatten().find(|e| e.path().join("wireless").exists())).map(|e| e.path().join("device"));
                    let nm = wifi.as_deref().and_then(pci_name_of).unwrap_or_else(|| t("sensors.wifi"));
                    ((40, "wifi".into()), nm, "network")
                }
                "amdgpu" | "nouveau" | "radeon" | "i915" | "xe" => {
                    let id = format!("gpu-{}", dev.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default());
                    ((10, id), pci_name_of(&dev).unwrap_or_else(|| t("sensors.gpu")), "gpu")
                }
                "thinkpad" => ((50, "ec".into()), t("sensors.thinkpadEc"), "board"),
                "acpitz" | "acpi_fan" => ((60, "acpi".into()), t("sensors.acpi"), "board"),
                n if n.starts_with("BAT") || n == "AC" || n.starts_with("ucsi") || n.starts_with("hidpp") => continue,
                n => ((55, format!("hw-{n}")), n.to_string(), "board"),
            };
            let entry = devs.entry(dkey.clone()).or_insert_with(|| (dname, dkind, None, vec![]));
            let mut files: Vec<String> = std::fs::read_dir(&h).map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
            files.sort_by_key(|f| (f.chars().take_while(|c| c.is_alphabetic()).collect::<String>(), f.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse::<u32>().unwrap_or(0)));
            for f in files {
                let Some(base) = f.strip_suffix("_input").or_else(|| f.strip_suffix("_average").filter(|b| b.starts_with("power"))) else { continue };
                let idx: u32 = base.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0);
                let Some(raw) = num(&h.join(&f)) else { continue };
                let label_file = read(h.join(format!("{base}_label"))).filter(|l| !l.is_empty());
                let hid = h.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let id = format!("{}:{hid}:{base}", dkey.1);
                if base.starts_with("temp") {
                    let v = raw / 1000.0;
                    if !(5.0..=150.0).contains(&v) {
                        continue;
                    }
                    let crit = num(&h.join(format!("{base}_crit"))).map(|c| c / 1000.0).filter(|c| *c > 0.0 && *c < 200.0);
                    let (warn, crit_t) = temp_limits(crit);
                    let (label, sub, order) = match (name.as_str(), label_file.as_deref()) {
                        ("coretemp", Some(l)) if l.starts_with("Package id") => (t("sensors.package"), None, 0),
                        ("coretemp", Some(l)) if l.starts_with("Core ") => {
                            let cid: u32 = l[5..].trim().parse().unwrap_or(0);
                            match self.cores.get(&(0, cid)).or_else(|| self.cores.iter().find(|(k, _)| k.1 == cid).map(|(_, c)| c)) {
                                Some(c) => (c.label.clone(), Some(Self::cpus_sub(&c.cpus)), c.order),
                                None => (l.to_string(), None, 500 + cid),
                            }
                        }
                        ("k10temp" | "zenpower", Some(l)) => (l.to_string(), None, idx),
                        ("nvme", Some("Composite")) => (t("sensors.composite"), None, 0),
                        ("nvme", Some(l)) if l.starts_with("Sensor ") => (tf("sensors.sensor", &[("n", l[7..].to_string())]), None, idx),
                        ("spd5118" | "jc42", _) => {
                            dimm += 1;
                            let addr = devs_str.rsplit('-').next().map(|a| format!("0x{}", a.trim_start_matches('0')));
                            (tf("sensors.dimm", &[("n", dimm.to_string())]), addr.map(|a| tf("sensors.address", &[("addr", a)])), dimm)
                        }
                        ("acpitz", _) => (tf("sensors.zone", &[("n", idx.to_string())]), Some("acpitz".into()), idx),
                        (_, Some(l)) => (l.to_string(), None, idx),
                        (n, None) if n.starts_with("iwlwifi") || n.starts_with("mt79") || n.starts_with("ath1") || n.starts_with("rtw") => (t("sensors.temperature"), None, 0),
                        (_, None) => (tf("sensors.sensor", &[("n", idx.to_string())]), None, idx),
                    };
                    entry.3.push(Raw { id, label, sub, kind: "temp", value: v, warn, crit: crit_t, order });
                } else if base.starts_with("fan") {
                    let label = label_file.unwrap_or_else(|| tf("sensors.fan", &[("n", idx.to_string())]));
                    entry.3.push(Raw { id, label, sub: None, kind: "fan", value: raw, warn: None, crit: None, order: 1000 + idx });
                } else if base.starts_with("in") {
                    let label = label_file.unwrap_or_else(|| tf("sensors.voltage", &[("n", idx.to_string())]));
                    entry.3.push(Raw { id, label, sub: None, kind: "volt", value: raw / 1000.0, warn: None, crit: None, order: 2000 + idx });
                } else if base.starts_with("curr") {
                    let label = label_file.unwrap_or_else(|| tf("sensors.current", &[("n", idx.to_string())]));
                    entry.3.push(Raw { id, label, sub: None, kind: "current", value: raw / 1000.0, warn: None, crit: None, order: 3000 + idx });
                } else if base.starts_with("power") {
                    let label = label_file.unwrap_or_else(|| t("sensors.power"));
                    entry.3.push(Raw { id, label, sub: None, kind: "power", value: raw / 1_000_000.0, warn: None, crit: None, order: 4000 + idx });
                }
            }
        }

        // ACPI thermal zones not already exposed through hwmon (the duplicate package sensor is skipped)
        let mut zones: Vec<PathBuf> = std::fs::read_dir("/sys/class/thermal").map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.file_name().map(|n| n.to_string_lossy().starts_with("thermal_zone")).unwrap_or(false)).collect()).unwrap_or_default();
        zones.sort_by_key(|p| p.file_name().and_then(|n| n.to_str()).and_then(|n| n.trim_start_matches("thermal_zone").parse::<u32>().ok()).unwrap_or(0));
        for z in zones {
            let zname = z.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if seen_zones.contains(&zname) {
                continue;
            }
            let ty = read(z.join("type")).unwrap_or_default();
            if ty.starts_with("INT3400") || (ty == "x86_pkg_temp" && has_coretemp) || ty == "acpitz" || ty.starts_with("iwlwifi") {
                continue;
            }
            let Some(v) = num(&z.join("temp")).map(|v| v / 1000.0).filter(|v| (5.0..=150.0).contains(v)) else { continue };
            let label = match ty.as_str() {
                "TCPU" => t("sensors.zoneCpu"),
                "TCPU_PCI" => t("sensors.zoneCpuPci"),
                "x86_pkg_temp" => t("sensors.package"),
                s if s.starts_with("SEN") => tf("sensors.sensor", &[("n", s[3..].to_string())]),
                s => s.to_string(),
            };
            let (warn, crit) = temp_limits(None);
            let entry = devs.entry((60, "acpi".into())).or_insert_with(|| (t("sensors.acpi"), "board", None, vec![]));
            let order = 100 + zname.trim_start_matches("thermal_zone").parse::<u32>().unwrap_or(0);
            entry.3.push(Raw { id: format!("acpi:{zname}"), label, sub: Some(ty), kind: "temp", value: v, warn, crit, order });
        }

        // per-core clocks
        {
            let mut cores: Vec<&Core> = self.cores.values().collect();
            cores.sort_by_key(|c| c.order);
            let mut clocks = vec![];
            for c in cores {
                let khz: Vec<f64> = c.cpus.iter().filter_map(|n| num(Path::new(&format!("/sys/devices/system/cpu/cpu{n}/cpufreq/scaling_cur_freq")))).collect();
                if khz.is_empty() {
                    continue;
                }
                let mhz = khz.iter().copied().fold(0.0, f64::max) / 1000.0;
                clocks.push(Raw { id: format!("cpu:clock:{}", c.cpus[0]), label: c.label.clone(), sub: Some(Self::cpus_sub(&c.cpus)), kind: "clock", value: mhz, warn: None, crit: None, order: 5000 + c.order });
            }
            if !clocks.is_empty() {
                let entry = devs.entry((0, "cpu".into())).or_insert_with(|| (cpu_name.clone(), "cpu", None, vec![]));
                entry.3.extend(clocks);
            }
            if let Some(s) = snap {
                let entry = devs.entry((0, "cpu".into())).or_insert_with(|| (cpu_name.clone(), "cpu", None, vec![]));
                entry.3.push(Raw { id: "cpu:usage".into(), label: t("sensors.usage"), sub: None, kind: "percent", value: s.cpu.total, warn: None, crit: None, order: 9000 });
            }
        }

        // GPUs from the snapshot (the NVIDIA one is never woken up just to read it)
        if let Some(s) = snap {
            for g in &s.gpus {
                let key = (10, format!("gpu{}", g.idx));
                let note = g.suspended.then(|| t("sensors.suspended"));
                let entry = devs.entry(key.clone()).or_insert_with(|| (g.name.clone(), "gpu", note, vec![]));
                if let Some(tc) = g.temp {
                    let (warn, crit) = temp_limits(None);
                    entry.3.push(Raw { id: format!("{}:temp", key.1), label: t("sensors.temperature"), sub: None, kind: "temp", value: tc, warn, crit, order: 0 });
                }
                if !g.suspended {
                    entry.3.push(Raw { id: format!("{}:util", key.1), label: t("sensors.usage"), sub: None, kind: "percent", value: g.util, warn: None, crit: None, order: 10 });
                    entry.3.push(Raw { id: format!("{}:mem", key.1), label: t("sensors.gpuMemory"), sub: None, kind: "memory", value: g.mem_used as f64, warn: None, crit: None, order: 20 });
                }
            }
        }

        // batteries
        if let Ok(rd) = std::fs::read_dir("/sys/class/power_supply") {
            let mut bats: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| read(p.join("type")).as_deref() == Some("Battery") && read(p.join("scope")).as_deref() != Some("Device")).collect();
            bats.sort();
            for b in bats {
                let bid = b.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                let maker = read(b.join("manufacturer")).unwrap_or_default();
                let model = read(b.join("model_name")).unwrap_or_default();
                let name = format!("{maker} {model}").trim().to_string();
                let status = read(b.join("status")).unwrap_or_default();
                let note = Some(match status.as_str() {
                    "Charging" => t("sensors.charging"),
                    "Discharging" => t("sensors.discharging"),
                    "Full" => t("sensors.full"),
                    _ => t("sensors.notCharging"),
                });
                let entry = devs.entry((70, bid.clone())).or_insert_with(|| (if name.is_empty() { t("sensors.battery") } else { tf("sensors.batteryNamed", &[("name", name)]) }, "battery", note, vec![]));
                let mut push = |id: &str, key: &str, kind: &'static str, v: Option<f64>, order: u32| {
                    if let Some(v) = v {
                        entry.3.push(Raw { id: format!("{bid}:{id}"), label: t(key), sub: None, kind, value: v, warn: None, crit: None, order });
                    }
                };
                push("charge", "sensors.charge", "percent", num(&b.join("capacity")), 0);
                push("voltage", "sensors.batteryVoltage", "volt", num(&b.join("voltage_now")).map(|v| v / 1e6), 1);
                push("power", "sensors.batteryPower", "power", num(&b.join("power_now")).map(|v| v / 1e6), 2);
                push("energy", "sensors.energy", "energy", num(&b.join("energy_now")).map(|v| v / 1e6), 3);
                let full = num(&b.join("energy_full"));
                let design = num(&b.join("energy_full_design"));
                push("wear", "sensors.wear", "percent", full.zip(design).filter(|(_, d)| *d > 0.0).map(|(f, d)| (100.0 - f / d * 100.0).max(0.0)), 4);
                push("cycles", "sensors.cycles", "count", num(&b.join("cycle_count")).filter(|c| *c > 0.0), 5);
            }
        }

        let mut out = vec![];
        for ((_, id), (name, kind, note, mut raws)) in devs {
            raws.sort_by_key(|r| r.order);
            let readings = raws
                .into_iter()
                .map(|r| {
                    let st = self.stats.entry(r.id.clone()).or_insert(Stat { min: r.value, max: r.value, sum: 0.0, n: 0 });
                    st.min = st.min.min(r.value);
                    st.max = st.max.max(r.value);
                    st.sum += r.value;
                    st.n += 1;
                    let value = if r.kind == "temp" {
                        match self.smooth.get_mut(&r.id) {
                            Some(s) => s.push(r.value),
                            None => {
                                self.smooth.insert(r.id.clone(), Smooth::new(r.value));
                                r.value
                            }
                        }
                    } else {
                        r.value
                    };
                    Reading { avg: st.sum / st.n as f64, min: st.min, max: st.max, id: r.id, label: r.label, sub: r.sub, kind: r.kind, value, warn: r.warn, crit: r.crit }
                })
                .collect::<Vec<_>>();
            if readings.is_empty() && note.is_none() {
                continue;
            }
            out.push(Device { id, name, kind, note, readings });
        }
        out
    }
}

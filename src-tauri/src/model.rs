//! Contract for the data sent to the UI (see src/lib/types.ts).
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Section {
    App,
    Ctr,
    Bg,
    Sys,
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Running,
    Stopped,
    Efficiency,
    Paused,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Proc {
    pub pid: i32,
    pub ctime: u64,
    // static fields: sent to the UI only the first time it sees the process (see wire())
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub user: String,
    pub cpu: f64,
    pub mem: u64,
    pub disk: Option<f64>,
    // most common values omitted: the UI rebuilds them (gpu 0, no engine, priority 0)
    #[serde(skip_serializing_if = "is_zero")]
    pub gpu: f64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub gpu_engine: String,
    pub status: Status,
    #[serde(skip_serializing_if = "is_zero_i")]
    pub nice: i32,
    pub threads: u32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub exe: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cmd: String,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

fn is_zero_i(v: &i32) -> bool {
    *v == 0
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub key: String,
    pub label: String,
    pub icon: Option<String>,
    pub section: Section,
    pub unit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    pub cpu: f64,
    pub mem: u64,
    pub disk: Option<f64>,
    pub gpu: f64,
    pub gpu_engine: String,
    pub status: Status,
    pub procs: Vec<Proc>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Disk {
    pub name: String,
    pub model: String,
    pub kind: String,
    pub active: f64,
    pub read: f64,
    pub write: f64,
    pub resp: f64,
    pub capacity: u64,
    pub system: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Net {
    pub name: String,
    pub kind: String,
    pub connected: bool,
    pub ssid: Option<String>,
    pub send: f64,
    pub recv: f64,
    pub link: f64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Gpu {
    pub idx: usize,
    pub name: String,
    pub vendor: String,
    pub util: f64,
    pub engines: BTreeMap<String, f64>,
    pub mem_used: u64,
    pub mem_total: Option<u64>,
    pub temp: Option<f64>,
    pub suspended: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Culprit {
    pub key: String,
    pub label: String,
    pub kind: String,
    pub container: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Insight {
    pub res: String,
    pub level: String,
    pub title: String,
    pub sentence: String,
    pub culprit: Option<Culprit>,
    pub others: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Cpu {
    pub total: f64,
    pub kernel: f64,
    pub iowait: f64,
    pub per_core: Vec<f64>,
    pub freq: f64,
    pub temp: Option<f64>,
    pub model: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Mem {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub pct: f64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub cached: u64,
    pub swap_in: f64,
    pub swap_out: f64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub procs: usize,
    pub threads: u64,
    pub handles: u64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub t: f64,
    pub cpu: Cpu,
    pub mem: Mem,
    pub disks: Vec<Disk>,
    pub nets: Vec<Net>,
    pub gpus: Vec<Gpu>,
    pub groups: Vec<Group>,
    pub insights: Vec<Insight>,
    pub counts: Counts,
    pub uptime: f64,
}

fn r2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

impl Snapshot {
    /// Lightweight copy for the UI: rounded numbers, and static process fields only for
    /// the processes the UI has not seen yet (`sent` is updated).
    pub fn wire(&self, sent: &mut std::collections::HashSet<(i32, u64)>) -> Snapshot {
        let mut s = self.clone();
        let mut live = std::collections::HashSet::with_capacity(sent.len() + 16);
        for g in &mut s.groups {
            g.cpu = r2(g.cpu);
            g.gpu = r2(g.gpu);
            g.disk = g.disk.map(f64::round);
            for p in &mut g.procs {
                p.cpu = r2(p.cpu);
                p.gpu = r2(p.gpu);
                p.disk = p.disk.map(f64::round);
                let k = (p.pid, p.ctime);
                live.insert(k);
                if !sent.insert(k) {
                    p.name.clear();
                    p.icon = None;
                    p.user.clear();
                    p.exe.clear();
                    p.cmd.clear();
                }
            }
        }
        sent.retain(|k| live.contains(k));
        for v in &mut s.cpu.per_core {
            *v = (*v * 10.0).round() / 10.0;
        }
        s.cpu.total = r2(s.cpu.total);
        s.cpu.kernel = r2(s.cpu.kernel);
        s
    }
}

//! Process scan from /proc: CPU, private memory, I/O, cached static data.
use crate::util::read_raw;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub type Key = (i32, u64); // (pid, start time in ticks)

pub struct Static {
    pub name: String,
    pub cmd: String,
    pub exe: String,
    pub uid: u32,
    pub user: String,
    pub kernel: bool,
    pub cgroup: String,
    pub io_ok: bool,
}

pub struct Sample {
    pub key: Key,
    pub pid: i32,
    pub ppid: i32,
    pub state: u8,
    pub nice: i32,
    pub threads: u32,
    pub cpu: f64,
    pub mem: u64,
    pub disk: Option<f64>,
    pub gpu: f64,
    pub gpu_engine: String,
    pub gpu_mem: u64,
}

pub struct Procs {
    clk: f64,
    page: u64,
    ncpu: f64,
    my_uid: u32,
    pub statics: HashMap<Key, Static>,
    prev_cpu: HashMap<Key, f64>,
    prev_io: HashMap<Key, u64>,
    users: HashMap<u32, String>,
    tick: u64,
}

impl Procs {
    pub fn new(ncpu: usize) -> Self {
        let clk = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
        Procs {
            clk: if clk > 0.0 { clk } else { 100.0 },
            page: if page > 0 { page } else { 4096 },
            ncpu: ncpu.max(1) as f64,
            my_uid: unsafe { libc::getuid() },
            statics: HashMap::new(),
            prev_cpu: HashMap::new(),
            prev_io: HashMap::new(),
            users: HashMap::new(),
            tick: 0,
        }
    }

    fn username(&mut self, uid: u32) -> String {
        if let Some(n) = self.users.get(&uid) {
            return n.clone();
        }
        if let Some(txt) = read_raw("/etc/passwd") {
            for l in txt.lines() {
                let f: Vec<&str> = l.split(':').collect();
                if f.len() > 2 {
                    if let Ok(u) = f[2].parse::<u32>() {
                        self.users.insert(u, f[0].to_string());
                    }
                }
            }
        }
        self.users.entry(uid).or_insert_with(|| uid.to_string()).clone()
    }

    fn read_static(&mut self, pid: i32, comm: &str, ppid: i32) -> Static {
        let base = format!("/proc/{pid}");
        let cmd = std::fs::read(format!("{base}/cmdline"))
            .map(|b| {
                b.split(|c| *c == 0)
                    .filter(|a| !a.is_empty())
                    .map(|a| String::from_utf8_lossy(a).into_owned())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();
        let exe = std::fs::read_link(format!("{base}/exe"))
            .map(|p| p.to_string_lossy().trim_end_matches(" (deleted)").to_string())
            .unwrap_or_default();
        let uid = std::fs::metadata(&base).map(|m| std::os::unix::fs::MetadataExt::uid(&m)).unwrap_or(u32::MAX);
        let kernel = (pid == 2 || ppid == 2) && cmd.is_empty();
        let mut name = comm.to_string();
        if name.len() == 15 && !cmd.is_empty() {
            let first = cmd.split(' ').next().unwrap_or("");
            if let Some(b) = Path::new(first).file_name() {
                let b = b.to_string_lossy();
                if b.starts_with(&name) {
                    name = b.into_owned();
                }
            }
        }
        let cgroup = read_raw(format!("{base}/cgroup"))
            .and_then(|t| t.lines().find_map(|l| l.strip_prefix("0::").map(|s| s.to_string())))
            .unwrap_or_default();
        let user = if uid == u32::MAX { "?".into() } else { self.username(uid) };
        let cmd = if cmd.len() > 400 { format!("{}…", &cmd[..cmd.char_indices().nth(400).map(|c| c.0).unwrap_or(cmd.len())]) } else { cmd };
        Static { name, cmd, exe, uid, user, kernel, cgroup, io_ok: uid == self.my_uid || self.my_uid == 0 }
    }

    pub fn scan(&mut self, dt: Option<f64>) -> Vec<Sample> {
        self.tick += 1;
        let mut out = Vec::with_capacity(700);
        let mut seen = HashSet::with_capacity(700);
        let Ok(rd) = std::fs::read_dir("/proc") else { return out };
        for e in rd.flatten() {
            let fname = e.file_name();
            let Some(pid) = fname.to_str().and_then(|s| s.parse::<i32>().ok()) else { continue };
            let Ok(stat) = std::fs::read(format!("/proc/{pid}/stat")) else { continue };
            let Some(rp) = stat.iter().rposition(|c| *c == b')') else { continue };
            let lp = stat.iter().position(|c| *c == b'(').unwrap_or(0);
            let rest = String::from_utf8_lossy(&stat[rp + 2..]).into_owned();
            let f: Vec<&str> = rest.split_whitespace().collect();
            if f.len() < 22 {
                continue;
            }
            let state = f[0].as_bytes()[0];
            if state == b'Z' || state == b'X' {
                continue;
            }
            let ppid: i32 = f[1].parse().unwrap_or(0);
            let utime: f64 = f[11].parse().unwrap_or(0.0);
            let stime: f64 = f[12].parse().unwrap_or(0.0);
            let nice: i32 = f[16].parse().unwrap_or(0);
            let threads: u32 = f[17].parse().unwrap_or(1);
            let start: u64 = f[19].parse().unwrap_or(0);
            let key = (pid, start);
            seen.insert(key);
            if !self.statics.contains_key(&key) {
                let comm = String::from_utf8_lossy(&stat[lp + 1..rp]).into_owned();
                let st = self.read_static(pid, &comm, ppid);
                self.statics.insert(key, st);
            }
            let (kernel, io_ok) = {
                let st = &self.statics[&key];
                (st.kernel, st.io_ok)
            };
            let cpu_secs = (utime + stime) / self.clk;
            let mut mem = 0;
            if !kernel {
                if let Some(sm) = read_raw(format!("/proc/{pid}/statm")) {
                    let v: Vec<u64> = sm.split_whitespace().take(3).filter_map(|x| x.parse().ok()).collect();
                    if v.len() == 3 {
                        mem = v[1].saturating_sub(v[2]) * self.page;
                    }
                }
            }
            let mut disk = None;
            if io_ok && !kernel {
                match std::fs::read_to_string(format!("/proc/{pid}/io")) {
                    Ok(t) => {
                        let mut tot = 0u64;
                        for l in t.lines() {
                            if let Some(v) = l.strip_prefix("read_bytes: ").or_else(|| l.strip_prefix("write_bytes: ")) {
                                tot += v.trim().parse::<u64>().unwrap_or(0);
                            }
                        }
                        let prev = self.prev_io.insert(key, tot);
                        disk = Some(match (prev, dt) {
                            (Some(p), Some(dt)) if dt > 0.0 => tot.saturating_sub(p) as f64 / dt,
                            _ => 0.0,
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                        if let Some(st) = self.statics.get_mut(&key) {
                            st.io_ok = false;
                        }
                    }
                    Err(_) => {}
                }
            }
            let prev = self.prev_cpu.insert(key, cpu_secs);
            let cpu = match (prev, dt) {
                (Some(p), Some(dt)) if dt > 0.0 => ((cpu_secs - p) / (dt * self.ncpu) * 100.0).clamp(0.0, 100.0),
                _ => 0.0,
            };
            out.push(Sample { key, pid, ppid, state, nice, threads, cpu, mem, disk, gpu: 0.0, gpu_engine: String::new(), gpu_mem: 0 });
        }
        if self.tick % 5 == 0 {
            self.statics.retain(|k, _| seen.contains(k));
            self.prev_cpu.retain(|k, _| seen.contains(k));
            self.prev_io.retain(|k, _| seen.contains(k));
        }
        out
    }
}

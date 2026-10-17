//! Full collection: processes grouped by app/container/service, system resources, GPU.
pub mod gpu;
pub mod procs;
pub mod sys;

use crate::cgroup;
use crate::desktop::{DesktopDb, Icons};
use crate::i18n::{self, t, tf};
use crate::model::*;
use crate::util::{read, read_raw, run, unescape, which};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct Collector {
    procs: procs::Procs,
    sys: sys::Sys,
    gpus: gpu::Gpus,
    db: DesktopDb,
    db_t: Instant,
    icons: Icons,
    units: HashMap<(bool, String), String>,
    units_t: Option<Instant>,
    containers: HashMap<String, (String, String)>,
    containers_t: Option<Instant>,
    meta: HashMap<(String, bool, u8), (String, Option<String>)>,
    picons: HashMap<(String, String), Option<String>>,
    prev_gio: HashMap<String, u64>,
    prev_t: Option<Instant>,
    start: Instant,
    boot: f64,
    my_uid: u32,
    my_pid: i32,
    /// Language of the cached names: when it changes, the .desktop names and labels are rebuilt.
    lang: &'static str,
    /// Name of the Ctrl+Alt+Del screen in every language (its group is shown as the app itself).
    secure_names: Vec<String>,
    pub frozen: Vec<String>,
}

struct Acc {
    label: String,
    icon: Option<String>,
    section: Section,
    unit: String,
    image: Option<String>,
    procs: Vec<Proc>,
    unreadable: bool,
}

impl Collector {
    pub fn new() -> Self {
        let ncpu = read_raw("/proc/stat").map(|t| t.lines().filter(|l| l.starts_with("cpu") && !l.starts_with("cpu ")).count()).unwrap_or(1);
        let boot = read_raw("/proc/stat").and_then(|t| t.lines().find_map(|l| l.strip_prefix("btime ").and_then(|v| v.trim().parse::<f64>().ok()))).unwrap_or(0.0);
        Collector {
            procs: procs::Procs::new(ncpu),
            sys: sys::Sys::new(),
            gpus: gpu::Gpus::new(),
            db: DesktopDb::load(),
            db_t: Instant::now(),
            icons: Icons::new(true),
            units: HashMap::new(),
            units_t: None,
            containers: HashMap::new(),
            containers_t: None,
            meta: HashMap::new(),
            picons: HashMap::new(),
            prev_gio: HashMap::new(),
            prev_t: None,
            start: Instant::now(),
            boot,
            my_uid: unsafe { libc::getuid() },
            my_pid: std::process::id() as i32,
            lang: i18n::lang(),
            secure_names: i18n::all("app.secureScreen"),
            frozen: vec![],
        }
    }

    fn refresh_units(&mut self) {
        self.units_t = Some(Instant::now());
        for user in [false, true] {
            let mut args = vec![];
            if user {
                args.push("--user");
            }
            args.extend(["list-units", "--all", "--output=json", "--no-pager", "--plain"]);
            let out = run("systemctl", &args, Duration::from_secs(5));
            if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(&out) {
                for u in arr {
                    if let (Some(n), Some(d)) = (u.get("unit").and_then(|v| v.as_str()), u.get("description").and_then(|v| v.as_str())) {
                        self.units.insert((user, n.to_string()), d.to_string());
                    }
                }
            }
        }
        self.meta.clear();
    }

    fn refresh_containers(&mut self) {
        self.containers_t = Some(Instant::now());
        for tool in ["docker", "podman"] {
            if !which(tool) {
                continue;
            }
            let out = run(tool, &["ps", "--no-trunc", "--format", "{{.ID}}\t{{.Names}}\t{{.Image}}"], Duration::from_secs(4));
            for l in out.lines() {
                let f: Vec<&str> = l.split('\t').collect();
                if f.len() >= 3 {
                    self.containers.insert(f[0].to_string(), (f[1].to_string(), f[2].to_string()));
                }
            }
        }
    }

    fn group_meta(&mut self, unit: &str, user: bool, section: Section) -> (String, Option<String>) {
        let ck = (unit.to_string(), user, section as u8);
        if let Some(m) = self.meta.get(&ck) {
            return m.clone();
        }
        let appid = cgroup::appid(unit);
        let mut label;
        let mut icon_name = None;
        if let Some(e) = appid.as_deref().and_then(|a| self.db.lookup_appid(a)) {
            label = e.name.clone();
            icon_name = e.icon.clone();
        } else {
            let desc = self.units.get(&(user, unit.to_string())).cloned().unwrap_or_default();
            let u = unescape(unit);
            let base = u.rsplit_once('.').map(|x| x.0).unwrap_or(&u).to_string();
            label = if let Some(n) = unit.strip_prefix("session-").and_then(|s| s.strip_suffix(".scope")) {
                tf("groups.session", &[("n", n.to_string())])
            } else if unit == "init.scope" {
                if user { t("groups.systemdUser") } else { "systemd".into() }
            } else if unit.starts_with("user@") {
                t("groups.userManager")
            } else if !desc.is_empty() && desc != unit && !desc.starts_with('/') {
                desc
            } else {
                appid.clone().unwrap_or(base.clone())
            };
            if let Some(e) = self.db.lookup_exec(base.trim_start_matches("plasma-")) {
                icon_name = e.icon.clone();
            }
            if icon_name.is_none() {
                icon_name = service_icon(&base.to_lowercase()).map(String::from);
            }
        }
        if label.is_empty() {
            label = unit.to_string();
        }
        // reverse-DNS style id without .desktop (e.g. org.chromium.Chromium) → last segment
        if !label.contains(' ') && label.matches('.').count() >= 2 {
            if let Some(last) = label.rsplit('.').next().filter(|s| !s.is_empty()) {
                let mut c = last.chars();
                label = c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default();
            }
        }
        let icon = icon_name.and_then(|n| if n.starts_with("glyph:") { Some(n) } else { self.icons.resolve(&n) });
        let m = (label, icon);
        self.meta.insert(ck, m.clone());
        m
    }

    fn proc_icon(&mut self, exe: &str, name: &str) -> Option<String> {
        let base = std::path::Path::new(exe).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let icon = self.db.lookup_exec(&base).or_else(|| self.db.lookup_exec(name)).and_then(|e| e.icon.clone())?;
        self.icons.resolve(&icon)
    }

    /// Icon of a single process: from its .desktop, from the theme by name, or from the known services map.
    fn any_proc_icon(&mut self, exe: &str, name: &str) -> Option<String> {
        let k = (exe.to_string(), name.to_string());
        if let Some(v) = self.picons.get(&k) {
            return v.clone();
        }
        let lname = name.to_lowercase();
        let v = self
            .proc_icon(exe, name)
            .or_else(|| self.icons.resolve(&lname))
            .or_else(|| service_icon(&lname).and_then(|n| if n.starts_with("glyph:") { Some(n.to_string()) } else { self.icons.resolve(n) }));
        self.picons.insert(k, v.clone());
        v
    }

    fn group_io(&mut self, path: &str, dt: Option<f64>) -> Option<f64> {
        let txt = read_raw(format!("/sys/fs/cgroup{path}/io.stat"))?;
        let mut tot = 0u64;
        for l in txt.lines() {
            for tok in l.split_whitespace().skip(1) {
                if let Some(v) = tok.strip_prefix("rbytes=").or_else(|| tok.strip_prefix("wbytes=")) {
                    tot += v.parse::<u64>().unwrap_or(0);
                }
            }
        }
        let prev = self.prev_gio.insert(path.to_string(), tot);
        match (prev, dt) {
            (Some(p), Some(dt)) if dt > 0.0 => Some(tot.saturating_sub(p) as f64 / dt),
            _ => None,
        }
    }

    pub fn collect(&mut self) -> Snapshot {
        let now = Instant::now();
        let dt = self.prev_t.map(|t| now.duration_since(t).as_secs_f64());
        self.prev_t = Some(now);
        if self.db_t.elapsed() > Duration::from_secs(120) || self.lang != i18n::lang() {
            self.lang = i18n::lang();
            self.db = DesktopDb::load();
            self.db_t = now;
            self.meta.clear();
            self.picons.clear();
        }
        if self.units_t.map(|t| t.elapsed() > Duration::from_secs(30)).unwrap_or(true) {
            self.refresh_units();
        }

        let cpu = self.sys.cpu();
        let mem = self.sys.mem();
        let disks = self.sys.disks(dt);
        let nets = self.sys.nets(dt);
        let mut samples = self.procs.scan(dt);

        let owners: HashMap<procs::Key, (u32, bool, String)> = samples
            .iter()
            .filter_map(|s| self.procs.statics.get(&s.key).map(|st| (s.key, (st.uid, st.kernel, st.name.clone()))))
            .collect();
        let gpus = self.gpus.collect(&mut samples, &owners);

        let threads: u64 = samples.iter().map(|s| s.threads as u64).sum();
        let mut nprocs = 0usize;
        let mut accs: HashMap<String, Acc> = HashMap::new();
        let mut order: Vec<String> = vec![];
        let my_uid_s = self.my_uid.to_string();

        for s in &samples {
            let Some(st) = self.procs.statics.get(&s.key) else { continue };
            if st.kernel {
                continue;
            }
            nprocs += 1;
            let (cg_path, uid, name, exe, cmd, user, io_ok) = (st.cgroup.clone(), st.uid, st.name.clone(), st.exe.clone(), st.cmd.clone(), st.user.clone(), st.io_ok);
            let cg = cgroup::parse(&cg_path);
            let unit = cg.unit.clone().unwrap_or_default();
            let ctr = cgroup::is_container(&unit);
            let app = !unit.is_empty() && cg.user_manager.is_some() && cgroup::is_app_unit(&unit);
            let section = if ctr {
                Section::Ctr
            } else if app {
                Section::App
            } else if cg.user_manager.as_deref() == Some(my_uid_s.as_str()) || uid == self.my_uid {
                Section::Bg
            } else {
                Section::Sys
            };
            let (key, label, icon, image) = if ctr {
                let cid = unit.split_once('-').map(|x| x.1).unwrap_or("").trim_end_matches(".scope").to_string();
                if !self.containers.contains_key(&cid) && self.containers_t.map(|t| t.elapsed() > Duration::from_secs(5)).unwrap_or(true) {
                    self.refresh_containers();
                }
                let (n, img) = self.containers.get(&cid).cloned().unwrap_or_else(|| (cid.chars().take(12).collect(), String::new()));
                (cg.unit_path.clone(), n, None, (!img.is_empty()).then_some(img))
            } else if !unit.is_empty() {
                let (label, icon) = self.group_meta(&unit, cg.user_manager.is_some(), section);
                let key = if app { format!("app:{}:{}", cg.user_manager.clone().unwrap_or_default(), label.to_lowercase()) } else { cg.unit_path.clone() };
                (key, label, icon, None)
            } else {
                let icon = self.proc_icon(&exe, &name);
                (format!("pid:{}", s.pid), name.clone(), icon, None)
            };
            let status = if s.state == b'T' || s.state == b't' {
                Status::Stopped
            } else if s.nice >= 19 {
                Status::Efficiency
            } else {
                Status::Running
            };
            let acc = accs.entry(key.clone()).or_insert_with(|| {
                order.push(key.clone());
                Acc { label, icon, section, unit: unit.clone(), image, procs: vec![], unreadable: false }
            });
            if !io_ok {
                acc.unreadable = true;
            }
            let picon = self.any_proc_icon(&exe, &name);
            if acc.icon.is_none() && acc.procs.is_empty() {
                acc.icon = picon.clone();
            }
            acc.procs.push(Proc {
                pid: s.pid,
                ctime: s.key.1,
                icon: picon,
                name,
                user,
                cpu: s.cpu,
                mem: s.mem,
                disk: s.disk,
                gpu: s.gpu,
                gpu_engine: s.gpu_engine.clone(),
                status,
                nice: s.nice,
                threads: s.threads,
                exe,
                cmd,
            });
        }

        self.frozen.clear();
        let mut groups = Vec::with_capacity(accs.len());
        for key in order {
            let Some(mut a) = accs.remove(&key) else { continue };
            a.procs.sort_by_key(|p| p.ctime);
            let cpu_sum: f64 = a.procs.iter().map(|p| p.cpu).sum();
            let mem_sum: u64 = a.procs.iter().map(|p| p.mem).sum();
            let gpu_sum = a.procs.iter().map(|p| p.gpu).sum::<f64>().min(100.0);
            let readable: Vec<f64> = a.procs.iter().filter_map(|p| p.disk).collect();
            let mut disk = if readable.is_empty() { None } else { Some(readable.iter().sum::<f64>()) };
            if key.starts_with('/') {
                if let Some(io) = self.group_io(&key, dt) {
                    if disk.is_none() || a.unreadable {
                        disk = Some(disk.unwrap_or(0.0).max(io));
                    }
                }
            }
            let engine = a.procs.iter().max_by(|x, y| x.gpu.total_cmp(&y.gpu)).filter(|p| p.gpu >= 0.05).map(|p| p.gpu_engine.clone()).unwrap_or_default();
            let frozen = a.section == Section::Ctr && read(format!("/sys/fs/cgroup{key}/cgroup.freeze")).as_deref() == Some("1");
            if frozen {
                self.frozen.push(key.clone());
            }
            let status = if frozen {
                Status::Paused
            } else if a.procs.iter().any(|p| p.status == Status::Stopped) {
                Status::Stopped
            } else if a.procs.iter().all(|p| p.status == Status::Efficiency) {
                Status::Efficiency
            } else {
                Status::Running
            };
            if a.icon.is_none() {
                a.icon = a.procs.iter().find_map(|p| p.icon.clone());
            }
            if a.procs.first().map(|p| p.pid) == Some(self.my_pid) || self.secure_names.iter().any(|n| a.label.starts_with(n.as_str())) {
                a.label = t("app.name");
            }
            groups.push(Group {
                key,
                label: a.label,
                icon: a.icon,
                section: a.section,
                unit: a.unit,
                image: a.image,
                cpu: cpu_sum,
                mem: mem_sum,
                disk,
                gpu: gpu_sum,
                gpu_engine: engine,
                status,
                procs: a.procs,
            });
        }
        if self.prev_gio.len() > groups.len() * 2 + 64 {
            let live: std::collections::HashSet<&String> = groups.iter().map(|g| &g.key).collect();
            self.prev_gio.retain(|k, _| live.contains(k));
        }

        let handles = read("/proc/sys/fs/file-nr").and_then(|v| v.split_whitespace().next().and_then(|x| x.parse().ok())).unwrap_or(0);
        let wall = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
        let mut snap = Snapshot {
            t: now.duration_since(self.start).as_secs_f64(),
            cpu,
            mem,
            disks,
            nets,
            gpus,
            groups,
            insights: vec![],
            counts: Counts { procs: nprocs, threads, handles },
            uptime: (wall - self.boot).max(0.0),
        };
        if dt.is_some() {
            snap.insights = crate::insights::analyze(&snap);
        }
        snap
    }
}

/// Theme icons for common services and daemons, matched by keyword in the unit or process name.
fn service_icon(n: &str) -> Option<&'static str> {
    const MAP: &[(&[&str], &str)] = &[
        (&["docker", "containerd", "podman", "conmon", "crun", "runc"], "glyph:container"),
        (&["kwin"], "kwin"),
        (&["boltd", "thunderbolt"], "preferences-desktop-thunderbolt"),
        (&["lpmd", "nvidia-powerd", "energy"], "preferences-system-power-management"),
        (&["switcheroo", "nvidia"], "preferences-desktop-display"),
        (&["obex"], "preferences-system-bluetooth"),
        (&["snapper", "limine", "fusermount", "btrfs"], "drive-harddisk"),
        (&["arch-update", "pacman", "paru", "yay", "packagekit", "fwupd", "discover"], "system-software-update"),
        (&["xembedsniproxy", "xsettingsd"], "kde"),
        (&["dconf"], "preferences-system"),
        (&["fish", "bash", "zsh", "konsole", "alacritty", "kitty", "foot", "tmux"], "utilities-terminal"),
        (&["plasmashell"], "plasmashell"),
        (&["kglobalaccel", "ibus", "fcitx"], "preferences-desktop-keyboard"),
        (&["kaccess", "at-spi", "orca"], "preferences-desktop-accessibility"),
        (&["ksystemstats", "ksysguard"], "utilities-system-monitor"),
        (&["baloo"], "baloo"),
        (&["kdeconnect"], "kdeconnect"),
        (&["xwayland", "colord"], "preferences-desktop-display"),
        (&["pipewire", "wireplumber", "pulseaudio", "rtkit"], "audio-card"),
        (&["bluetooth", "bluez"], "preferences-system-bluetooth"),
        (&["tailscale", "wireguard", "openvpn", "zerotier"], "network-vpn"),
        (&["networkmanager", "wpa_supplicant", "iwd", "networkd", "resolved", "avahi", "dnsmasq", "nm-"], "preferences-system-network"),
        (&["cups"], "cups"),
        (&["upower", "powerdevil", "power-profiles", "tlp", "thermald"], "preferences-system-power-management"),
        (&["polkit", "keyring", "kwallet", "ksecretd", "gpg-agent", "ssh-agent", "secret"], "dialog-password"),
        (&["udisks", "smartd", "fstrim", "lvm", "mdadm"], "drive-harddisk"),
        (&["accounts-daemon", "userdb", "logind", "session-", "user@"], "preferences-system-users"),
        (&["libvirt", "virtqemud", "virtlog", "qemu"], "virt-manager"),
        (&["sddm", "getty", "login", "gdm"], "preferences-system-login"),
        (&["ananicy", "irqbalance", "scx", "gamemode", "earlyoom", "oomd"], "preferences-system-performance"),
        (&["timesyncd", "chrony", "ntpd"], "preferences-system-time"),
        (&["firewalld", "ufw", "nftables", "iptables"], "preferences-security-firewall"),
        (&["cron", "atd", "anacron"], "clock"),
        (&["gvfs", "tracker", "localsearch"], "system-file-manager"),
        (&["xdg-desktop-portal", "xdg-permission", "xdg-document"], "kde"),
        (&["kded", "kactivitymanager", "ksmserver", "kiod", "kioworker", "kscreen", "plasma", "kde"], "kde"),
        (&["dbus"], "applications-system"),
        (&["systemd", "init", "udev", "journal"], "preferences-system"),
    ];
    MAP.iter().find(|(keys, _)| keys.iter().any(|k| n.contains(k))).map(|(_, v)| *v)
}

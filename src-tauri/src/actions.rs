//! Actions on processes, groups and containers. Every signal checks the process start time
//! so it does not hit a PID reused by another program.
use crate::model::{Section, Snapshot};
use crate::util::{read_raw, which};
use serde_json::Value;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

pub type Last = Arc<Mutex<Option<Snapshot>>>;

fn start_time(pid: i32) -> Option<u64> {
    let stat = read_raw(format!("/proc/{pid}/stat"))?;
    let rest = &stat[stat.rfind(')')? + 2..];
    rest.split_whitespace().nth(19)?.parse().ok()
}

fn spawn(cmd: &str, args: &[&str]) -> Result<(), String> {
    Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Impossibile avviare {cmd}: {e}"))
}

fn sig_of(name: &str) -> i32 {
    match name {
        "KILL" => libc::SIGKILL,
        "STOP" => libc::SIGSTOP,
        "CONT" => libc::SIGCONT,
        _ => libc::SIGTERM,
    }
}

/// Sends a signal; if permission is missing (other users' processes) asks for the password via pkexec.
fn signal(targets: &[(i32, u64)], sig: &str) -> Result<String, String> {
    let s = sig_of(sig);
    let mut denied = vec![];
    let mut done = 0;
    for &(pid, ctime) in targets {
        if pid == std::process::id() as i32 && s == libc::SIGSTOP {
            continue;
        }
        if start_time(pid) != Some(ctime) {
            continue;
        }
        let r = unsafe { libc::kill(pid, s) };
        if r == 0 {
            done += 1;
        } else if std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM) {
            denied.push(pid.to_string());
        }
    }
    if !denied.is_empty() && which("pkexec") {
        let mut args = vec!["kill", "-s", sig];
        args.extend(denied.iter().map(|s| s.as_str()));
        spawn("pkexec", &args)?;
        return Ok(format!("Richiesta l'autorizzazione per {} processi di altri utenti", denied.len()));
    }
    Ok(format!("{done} processi"))
}

fn set_efficiency(targets: &[(i32, u64)], on: bool) -> Result<String, String> {
    let mut denied = vec![];
    for &(pid, ctime) in targets {
        if start_time(pid) != Some(ctime) {
            continue;
        }
        let r = unsafe { libc::setpriority(libc::PRIO_PROCESS, pid as libc::id_t, if on { 19 } else { 0 }) };
        if r != 0 {
            denied.push(pid.to_string());
        }
        // I/O: "idle" class in efficiency mode, "best effort" otherwise
        let prio = if on { 3 << 13 } else { (2 << 13) | 4 };
        unsafe { libc::syscall(libc::SYS_ioprio_set, 1, pid, prio) };
    }
    if !denied.is_empty() && which("pkexec") {
        let n = if on { "19" } else { "0" };
        let mut args = vec!["renice", "-n", n, "-p"];
        args.extend(denied.iter().map(|s| s.as_str()));
        spawn("pkexec", &args)?;
    }
    Ok("ok".into())
}

fn container_cmd(unit: &str, verb: &str) -> Result<String, String> {
    let (tool, cid) = if let Some(r) = unit.strip_prefix("docker-") {
        ("docker", r.trim_end_matches(".scope"))
    } else if let Some(r) = unit.strip_prefix("libpod-") {
        ("podman", r.trim_end_matches(".scope"))
    } else {
        return Err("Non è un container".into());
    };
    match verb {
        "logs" | "shell" => {
            let inner = if verb == "logs" { format!("{tool} logs -f --tail 300 {cid}") } else { format!("{tool} exec -it {cid} sh -c 'command -v bash >/dev/null && exec bash || exec sh'") };
            for (term, pre) in [("konsole", vec!["--hold", "-e"]), ("kgx", vec!["--"]), ("gnome-terminal", vec!["--"]), ("alacritty", vec!["-e"])] {
                if which(term) {
                    let mut a: Vec<&str> = pre;
                    a.extend(["sh", "-c", &inner]);
                    return spawn(term, &a).map(|_| "ok".into());
                }
            }
            Err("Nessun terminale trovato".into())
        }
        "pause" | "unpause" | "stop" | "restart" | "kill" => {
            let out = Command::new(tool).args([verb, cid]).output().map_err(|e| e.to_string())?;
            if out.status.success() {
                Ok("ok".into())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
            }
        }
        _ => Err(format!("Azione sconosciuta: {verb}")),
    }
}

fn open_location(path: &str) -> Result<String, String> {
    if path.is_empty() {
        return Err("Percorso non disponibile".into());
    }
    let uri = format!("file://{path}");
    let ok = Command::new("gdbus")
        .args(["call", "--session", "--dest", "org.freedesktop.FileManager1", "--object-path", "/org/freedesktop/FileManager1", "--method", "org.freedesktop.FileManager1.ShowItems", &format!("['{uri}']"), ""])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ok {
        return Ok("ok".into());
    }
    let dir = std::path::Path::new(path).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    spawn("xdg-open", &[&dir]).map(|_| "ok".into())
}

fn properties(pid: i32, group: Option<&crate::model::Group>) -> Result<String, String> {
    let base = format!("/proc/{pid}");
    let link = |n: &str| std::fs::read_link(format!("{base}/{n}")).map(|x| x.to_string_lossy().into_owned()).unwrap_or_default();
    let stat = read_raw(format!("{base}/stat")).ok_or("Il processo non esiste più")?;
    let comm = stat[stat.find('(').map(|i| i + 1).unwrap_or(0)..stat.rfind(')').unwrap_or(0)].to_string();
    let f: Vec<&str> = stat[stat.rfind(')').unwrap_or(0) + 2..].split_whitespace().collect();
    let clk = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
    let boot = read_raw("/proc/stat")
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("btime ").and_then(|v| v.trim().parse::<f64>().ok())))
        .unwrap_or(0.0);
    let started = boot + f.get(19).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0) / clk;
    let cmd = std::fs::read(format!("{base}/cmdline"))
        .map(|b| b.split(|c| *c == 0).filter(|a| !a.is_empty()).map(|a| String::from_utf8_lossy(a).into_owned()).collect::<Vec<_>>().join(" "))
        .unwrap_or_default();
    let status = read_raw(format!("{base}/status")).unwrap_or_default();
    let field = |k: &str| status.lines().find_map(|l| l.strip_prefix(k).map(|v| v.trim().to_string())).unwrap_or_default();
    let fds = std::fs::read_dir(format!("{base}/fd")).map(|d| d.count() as i64).unwrap_or(-1);
    let cgroup = read_raw(format!("{base}/cgroup")).and_then(|t| t.lines().find_map(|l| l.strip_prefix("0::").map(|s| s.to_string()))).unwrap_or_default();
    let v = serde_json::json!({
        "pid": pid,
        "ppid": f.get(1).and_then(|v| v.parse::<i32>().ok()),
        "name": comm,
        "exe": link("exe"),
        "cwd": link("cwd"),
        "cmd": cmd,
        "started": started,
        "threads": f.get(17).and_then(|v| v.parse::<u32>().ok()),
        "nice": f.get(16).and_then(|v| v.parse::<i32>().ok()),
        "uid": field("Uid:").split_whitespace().next().unwrap_or("").to_string(),
        "vmRss": field("VmRSS:"),
        "vmSwap": field("VmSwap:"),
        "fds": fds,
        "cgroup": cgroup,
        "group": group.map(|g| g.label.clone()),
        "unit": group.map(|g| g.unit.clone()),
    });
    Ok(v.to_string())
}

pub fn handle(last: &Last, action: &str, p: &Value) -> Result<String, String> {
    let snap = last.lock().map_err(|_| "stato non disponibile")?.clone();
    let key = p.get("key").and_then(|v| v.as_str()).unwrap_or("");
    // the key of a process row is "p:<pid>:<start>"
    let pkey: Option<(i32, u64)> = key.strip_prefix("p:").and_then(|r| {
        let (a, b) = r.split_once(':')?;
        Some((a.parse().ok()?, b.parse().ok()?))
    });
    let pid = p.get("pid").and_then(|v| v.as_i64()).map(|v| v as i32).or(pkey.map(|k| k.0));
    let p_ctime = p.get("ctime").and_then(|v| v.as_u64()).or(pkey.map(|k| k.1));
    let group = snap.as_ref().and_then(|s| s.groups.iter().find(|g| g.key == key).cloned());
    // targets: a single process (pid) or all processes in the group (key)
    let targets: Vec<(i32, u64)> = match (pid, &group) {
        (Some(pid), _) => {
            let ct = p_ctime.or_else(|| start_time(pid));
            ct.map(|c| vec![(pid, c)]).unwrap_or_default()
        }
        (None, Some(g)) => g.procs.iter().map(|x| (x.pid, x.ctime)).collect(),
        _ => vec![],
    };
    match action {
        "signal" => signal(&targets, p.get("sig").and_then(|v| v.as_str()).unwrap_or("TERM")),
        "terminate" => {
            if let Some(g) = &group {
                if g.section == Section::Ctr && pid.is_none() {
                    return container_cmd(&g.unit, "stop");
                }
            }
            let force = p.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
            signal(&targets, if force { "KILL" } else { "TERM" })
        }
        "suspend" => signal(&targets, if p.get("resume").and_then(|v| v.as_bool()).unwrap_or(false) { "CONT" } else { "STOP" }),
        "efficiency" => set_efficiency(&targets, p.get("on").and_then(|v| v.as_bool()).unwrap_or(true)),
        "container" => {
            let g = group.ok_or("Container non trovato")?;
            container_cmd(&g.unit, p.get("verb").and_then(|v| v.as_str()).unwrap_or(""))
        }
        "open-location" => {
            let exe = match (pid, &group) {
                (Some(pid), _) => std::fs::read_link(format!("/proc/{pid}/exe")).map(|x| x.to_string_lossy().into_owned()).unwrap_or_default(),
                (None, Some(g)) => g.procs.first().map(|x| x.exe.clone()).unwrap_or_default(),
                _ => String::new(),
            };
            open_location(&exe)
        }
        "properties" => {
            let pid = pid.or_else(|| group.as_ref().and_then(|g| g.procs.first().map(|x| x.pid))).ok_or("Processo non trovato")?;
            properties(pid, group.as_ref())
        }
        "run" => {
            let cmd = p.get("cmd").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if cmd.is_empty() {
                return Err("Scrivi un comando".into());
            }
            if p.get("admin").and_then(|v| v.as_bool()).unwrap_or(false) {
                return spawn("pkexec", &["sh", "-c", &cmd]).map(|_| "ok".into());
            }
            Command::new("sh")
                .args(["-c", &format!("setsid {cmd} >/dev/null 2>&1 &")])
                .current_dir(crate::util::home())
                .spawn()
                .map(|_| "ok".into())
                .map_err(|e| e.to_string())
        }
        _ => Err(format!("Azione sconosciuta: {action}")),
    }
}

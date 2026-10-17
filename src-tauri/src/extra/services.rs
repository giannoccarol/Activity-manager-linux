//! Servizi systemd del gestore di sistema e di quello dell'utente: unità caricate più quelle solo installate.
use crate::util::{config_home, data_home, read_raw, run, unescape};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    pub unit: String,
    pub description: String,
    pub scope: String,
    pub load: String,
    pub active: String,
    pub sub: String,
    pub state: String,
}

/// Stato di abilitazione per nome di unità: [sistema, utente].
type Files = [HashMap<String, String>; 2];

/// `list-unit-files` è lento (quasi un secondo): stati di abilitazione in cache per 30 s, azzerata dopo ogni azione.
static FILES: Mutex<Option<(Instant, Files)>> = Mutex::new(None);
/// Descrizioni delle unità installate ma non caricate (non cambiano durante la sessione).
static DESCS: Mutex<Option<HashMap<(bool, String), String>>> = Mutex::new(None);

fn systemctl_json(user: bool, args: &[&str]) -> Vec<Value> {
    let mut a = vec![];
    if user {
        a.push("--user");
    }
    a.extend(args);
    a.extend(["--output=json", "--no-pager"]);
    match serde_json::from_str(&run("systemctl", &a, Duration::from_secs(10))) {
        Ok(Value::Array(v)) => v,
        _ => vec![],
    }
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("")
}

fn unit_files(user: bool) -> HashMap<String, String> {
    systemctl_json(user, &["list-unit-files", "--type=service"]).iter().map(|f| (s(f, "unit_file").to_string(), s(f, "state").to_string())).collect()
}

fn unit_dirs(user: bool) -> Vec<PathBuf> {
    if user {
        let uid = unsafe { libc::getuid() };
        vec![config_home().join("systemd/user"), "/etc/systemd/user".into(), format!("/run/user/{uid}/systemd/user").into(), data_home().join("systemd/user"), "/usr/local/lib/systemd/user".into(), "/usr/lib/systemd/user".into()]
    } else {
        ["/etc/systemd/system", "/run/systemd/system", "/run/systemd/generator", "/usr/local/lib/systemd/system", "/usr/lib/systemd/system"].iter().map(PathBuf::from).collect()
    }
}

/// Description= letta dal file dell'unità; per quelle non trovate si chiede a systemctl in un'unica chiamata.
fn describe(user: bool, names: &[String]) -> HashMap<String, String> {
    let dirs = unit_dirs(user);
    let mut out = HashMap::new();
    let mut ask = vec![];
    for n in names {
        let desc = dirs.iter().find_map(|d| read_raw(d.join(n))).and_then(|t| {
            t.lines().map(str::trim).find_map(|l| l.strip_prefix("Description=").map(|v| v.trim().to_string()))
        });
        match desc {
            Some(d) if !d.contains('%') => {
                out.insert(n.clone(), d);
            }
            _ => ask.push(n.as_str()),
        }
    }
    if !ask.is_empty() {
        let mut a = vec![];
        if user {
            a.push("--user");
        }
        a.extend(["show", "-p", "Id", "-p", "Description", "--"]);
        a.extend(ask.iter().copied());
        for block in run("systemctl", &a, Duration::from_secs(10)).split("\n\n") {
            let get = |k: &str| block.lines().find_map(|l| l.strip_prefix(k));
            if let (Some(id), Some(d)) = (get("Id="), get("Description=")) {
                out.insert(id.to_string(), d.to_string());
            }
        }
    }
    for n in names {
        out.entry(n.clone()).or_default();
    }
    out
}

fn scope_list(user: bool, files: &HashMap<String, String>) -> Vec<Service> {
    let units = systemctl_json(user, &["list-units", "--all", "--type=service", "--plain"]);
    let loaded: HashMap<&str, &Value> = units.iter().map(|u| (s(u, "unit"), u)).filter(|(n, _)| n.ends_with(".service")).collect();
    let mut names: BTreeSet<&str> = loaded.keys().copied().collect();
    names.extend(files.keys().map(String::as_str).filter(|n| !n.ends_with("@.service")));

    let missing: Vec<String> = {
        let descs = DESCS.lock().unwrap_or_else(|e| e.into_inner());
        names.iter().filter(|n| !loaded.contains_key(*n) && !descs.as_ref().is_some_and(|d| d.contains_key(&(user, n.to_string())))).map(|n| n.to_string()).collect()
    };
    let found = if missing.is_empty() { HashMap::new() } else { describe(user, &missing) };
    let mut descs = DESCS.lock().unwrap_or_else(|e| e.into_inner());
    let descs = descs.get_or_insert_with(HashMap::new);
    descs.extend(found.into_iter().map(|(k, v)| ((user, k), v)));

    let mut out = vec![];
    for n in names {
        let u = loaded.get(n);
        let state = files.get(n).or_else(|| n.split_once('@').and_then(|(t, _)| files.get(&format!("{t}@.service"))));
        let load = u.map(|u| s(u, "load")).unwrap_or(if state.is_some() { "not-loaded" } else { "not-found" });
        let state = state.map(String::as_str).unwrap_or("");
        // come nella versione Python: niente unità inesistenti, alias e app dell'utente (stanno in Processi)
        if load == "not-found" || state == "alias" || n.starts_with("app-") {
            continue;
        }
        let description = u.map(|u| s(u, "description").to_string()).filter(|d| !d.is_empty()).or_else(|| descs.get(&(user, n.to_string())).cloned()).unwrap_or_default();
        out.push(Service {
            unit: n.to_string(),
            description,
            scope: if user { "user" } else { "system" }.into(),
            load: load.into(),
            active: u.map(|u| s(u, "active")).unwrap_or("inactive").into(),
            sub: u.map(|u| s(u, "sub")).unwrap_or("dead").into(),
            state: state.into(),
        });
    }
    out
}

pub fn list() -> Vec<Service> {
    let files = {
        let mut c = FILES.lock().unwrap_or_else(|e| e.into_inner());
        if c.as_ref().map(|(t, _)| t.elapsed() > Duration::from_secs(30)).unwrap_or(true) {
            let user = std::thread::spawn(|| unit_files(true));
            let system = unit_files(false);
            *c = Some((Instant::now(), [system, user.join().unwrap_or_default()]));
        }
        c.as_ref().map(|(_, f)| f.clone()).unwrap_or_default()
    };
    let [sys_files, user_files] = files;
    let user = std::thread::spawn(move || scope_list(true, &user_files));
    let mut out = scope_list(false, &sys_files);
    out.extend(user.join().unwrap_or_default());
    out.sort_by(|a, b| unescape(&a.unit).to_lowercase().cmp(&unescape(&b.unit).to_lowercase()).then_with(|| a.scope.cmp(&b.scope)));
    out
}

const VERBS: [(&str, &str, &str); 7] = [
    ("start", "avviato", "avviare"),
    ("stop", "arrestato", "arrestare"),
    ("restart", "riavviato", "riavviare"),
    ("enable", "abilitato all'avvio", "abilitare"),
    ("disable", "disabilitato all'avvio", "disabilitare"),
    ("mask", "mascherato", "mascherare"),
    ("unmask", "smascherato", "smascherare"),
];

fn valid_unit(u: &str) -> bool {
    u.len() <= 256
        && u.strip_suffix(".service").is_some_and(|b| !b.is_empty() && !b.starts_with('-'))
        && u.chars().all(|c| c.is_ascii_alphanumeric() || "@._:-\\".contains(c))
}

/// Esegue systemctl con timeout, restituendo (riuscito, stderr). Nessun pkexec: systemctl chiede a polkit,
/// che mostra la finestra di autenticazione di KDE.
fn systemctl(args: &[&str], timeout: Duration) -> Result<(bool, String), String> {
    let mut child = Command::new("systemctl")
        .args(args)
        .env("LC_ALL", "C")
        .env("SYSTEMD_COLORS", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Impossibile avviare systemctl: {e}"))?;
    let mut stderr = child.stderr.take();
    let reader = std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(e) = stderr.as_mut() {
            let _ = e.read_to_string(&mut buf);
        }
        buf
    });
    let start = Instant::now();
    let ok = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st.success(),
            Ok(None) if start.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("systemctl non ha risposto entro un minuto".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(format!("systemctl: {e}")),
        }
    };
    Ok((ok, reader.join().unwrap_or_default().trim().to_string()))
}

pub fn action(unit: &str, scope: &str, verb: &str) -> Result<String, String> {
    let Some(&(_, done, infinitive)) = VERBS.iter().find(|v| v.0 == verb) else {
        return Err(format!("Azione non consentita: {verb}"));
    };
    if !valid_unit(unit) {
        return Err(format!("Nome di servizio non valido: {unit}"));
    }
    let user = match scope {
        "user" => true,
        "system" => false,
        _ => return Err(format!("Contesto non valido: {scope}")),
    };
    let mut args = vec![];
    if user {
        args.push("--user");
    }
    args.extend([verb, "--", unit]);
    let res = systemctl(&args, Duration::from_secs(60));
    if matches!(verb, "enable" | "disable" | "mask" | "unmask") {
        *FILES.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
    let (ok, err) = res?;
    let name = unescape(unit.trim_end_matches(".service"));
    if ok {
        return Ok(format!("Servizio «{name}» {done}"));
    }
    let low = err.to_lowercase();
    if ["access denied", "authentication", "not authorized", "authorization", "dismissed", "permission denied"].iter().any(|k| low.contains(k)) {
        return Err(format!("Autorizzazione negata o annullata: impossibile {infinitive} «{name}»."));
    }
    let why = if low.contains("not found") || low.contains("does not exist") {
        "il servizio non esiste".to_string()
    } else if low.contains("not loaded") {
        "il servizio non è caricato".to_string()
    } else if low.contains("job for") && low.contains("failed") {
        format!("il servizio non è riuscito (dettagli con journalctl -xeu {unit})")
    } else {
        err
    };
    Err(if why.is_empty() { format!("Impossibile {infinitive} «{name}».") } else { format!("Impossibile {infinitive} «{name}»: {why}") })
}

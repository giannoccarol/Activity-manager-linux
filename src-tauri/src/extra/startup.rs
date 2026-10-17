//! App di avvio: voci XDG autostart dell'utente (~/.config/autostart) e di sistema (/etc/xdg/autostart).
//! Disabilitare una voce di sistema crea una copia con Hidden=true nella cartella dell'utente, come Impostazioni di sistema.
use crate::desktop::{parse_desktop, Icons};
use crate::util::config_home;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StartupItem {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub exec: String,
    pub comment: String,
    pub source: String,
    pub path: String,
    pub enabled: bool,
}

/// Segna le copie create da Gestione attività (stessa chiave della versione Python): riabilitandole si torna alla voce di sistema.
const MARK: &str = "X-Gestione-Attivita-Override";

fn user_dir() -> PathBuf {
    config_home().join("autostart")
}

fn system_dirs() -> Vec<PathBuf> {
    let dirs = std::env::var("XDG_CONFIG_DIRS").ok().filter(|v| !v.is_empty()).unwrap_or_else(|| "/etc/xdg".into());
    dirs.split(':').filter(|d| !d.is_empty()).map(|d| Path::new(d).join("autostart")).collect()
}

fn desktop_files(dir: &Path) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
    rd.flatten().filter_map(|e| e.file_name().into_string().ok()).filter(|n| n.ends_with(".desktop") && !n.starts_with('.')).collect()
}

fn system_file(id: &str) -> Option<PathBuf> {
    system_dirs().into_iter().map(|d| d.join(id)).find(|p| p.is_file())
}

fn is_true(kv: &HashMap<String, String>, key: &str) -> Option<bool> {
    kv.get(key).map(|v| v.trim().eq_ignore_ascii_case("true"))
}

pub fn list() -> Vec<StartupItem> {
    let mut entries: BTreeMap<String, (Option<PathBuf>, Option<PathBuf>)> = BTreeMap::new();
    // la prima cartella di XDG_CONFIG_DIRS ha la precedenza
    for d in system_dirs().iter().rev() {
        for f in desktop_files(d) {
            entries.entry(f.clone()).or_default().1 = Some(d.join(&f));
        }
    }
    let ud = user_dir();
    for f in desktop_files(&ud) {
        entries.entry(f.clone()).or_default().0 = Some(ud.join(&f));
    }
    let desk: Vec<String> = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "KDE".into()).split(':').map(String::from).collect();
    let shown = |v: Option<&String>| v.map(|v| v.split(';').any(|x| !x.is_empty() && desk.iter().any(|d| d == x)));
    let mut icons = Icons::new(true);
    let mut out = vec![];
    for (id, (user, system)) in entries {
        let Some(path) = user.clone().or(system.clone()) else { continue };
        let Some(kv) = parse_desktop(&path) else { continue };
        // copia dell'utente ridotta a "Hidden=true" (senza Exec): il resto dalla voce di sistema
        let base = system.as_deref().filter(|_| user.is_some() && !kv.contains_key("Exec")).and_then(parse_desktop).unwrap_or_default();
        let get = |k: &str| kv.get(k).or_else(|| base.get(k)).filter(|v| !v.is_empty()).cloned();
        if shown(get("OnlyShowIn").as_ref()) == Some(false) || shown(get("NotShowIn").as_ref()) == Some(true) {
            continue;
        }
        let hidden = is_true(&kv, "Hidden").unwrap_or(false);
        let gnome_off = is_true(&kv, "X-GNOME-Autostart-enabled") == Some(false);
        let stem = id.trim_end_matches(".desktop").to_string();
        let icon = get("Icon").and_then(|i| icons.resolve(&i)).or_else(|| icons.resolve(&stem));
        out.push(StartupItem {
            name: get("Name").unwrap_or_else(|| stem.clone()),
            icon,
            exec: get("Exec").unwrap_or_default(),
            comment: get("Comment").unwrap_or_default(),
            source: if system.is_some() { "system" } else { "user" }.into(),
            path: path.to_string_lossy().into_owned(),
            enabled: !(hidden || gnome_off),
            id,
        });
    }
    out.sort_by_key(|e| e.name.to_lowercase());
    out
}

/// Imposta (o aggiunge) una chiave nella sezione [Desktop Entry] lasciando invariato il resto del file.
fn set_key(text: &str, key: &str, value: &str) -> String {
    let mut out: Vec<String> = vec![];
    let (mut in_entry, mut done, mut seen_entry) = (false, false, false);
    for line in text.lines() {
        let s = line.trim();
        if s.starts_with('[') {
            if in_entry && !done {
                out.push(format!("{key}={value}"));
                done = true;
            }
            in_entry = s == "[Desktop Entry]";
            seen_entry |= in_entry;
        } else if in_entry && s.split_once('=').map(|(k, _)| k.trim() == key).unwrap_or(false) {
            if !done {
                out.push(format!("{key}={value}"));
                done = true;
            }
            continue;
        }
        out.push(line.to_string());
    }
    if !done {
        if !seen_entry {
            out.insert(0, "[Desktop Entry]".into());
        }
        out.push(format!("{key}={value}"));
    }
    out.join("\n") + "\n"
}

/// Scrittura atomica: un collegamento simbolico viene sostituito da un file vero, senza toccare la destinazione.
fn write(path: &Path, text: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("desktop.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

fn io_err(e: std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::PermissionDenied => "permesso negato".into(),
        std::io::ErrorKind::NotFound => "file non trovato".into(),
        _ => e.to_string(),
    }
}

pub fn set_enabled(id: &str, enabled: bool) -> Result<(), String> {
    let id = if id.ends_with(".desktop") { id.to_string() } else { format!("{id}.desktop") };
    if id.contains(['/', '\0']) || id.starts_with('.') {
        return Err("Voce di avvio non valida".into());
    }
    let ud = user_dir();
    let user = ud.join(&id);
    let own = user.symlink_metadata().is_ok().then(|| parse_desktop(&user).unwrap_or_default());
    let system = system_file(&id);
    // si modifica la copia dell'utente solo se è una voce completa e non una nostra copia della voce di sistema
    let src = match (&own, &system) {
        (Some(kv), Some(sys)) if !kv.contains_key("Exec") || is_true(kv, MARK) == Some(true) => sys.clone(),
        (Some(_), _) => user.clone(),
        (None, Some(sys)) => sys.clone(),
        (None, None) => return Err(format!("La voce di avvio «{id}» non esiste più")),
    };
    let from_system = src != user;
    let kv = parse_desktop(&src).unwrap_or_default();
    let name = kv.get("Name").cloned().unwrap_or_else(|| id.clone());
    let fail = |e: std::io::Error| format!("Impossibile modificare «{name}»: {}", io_err(e));
    let active = is_true(&kv, "Hidden") != Some(true) && is_true(&kv, "X-GNOME-Autostart-enabled") != Some(false);
    if from_system && enabled && active {
        // la voce di sistema è già attiva: basta togliere la copia dell'utente
        return if own.is_some() { std::fs::remove_file(&user).map_err(fail) } else { Ok(()) };
    }
    let mut text = set_key(&std::fs::read_to_string(&src).map_err(fail)?, "Hidden", if enabled { "false" } else { "true" });
    if from_system {
        text = set_key(&text, MARK, "true");
    }
    if enabled && is_true(&kv, "X-GNOME-Autostart-enabled") == Some(false) {
        text = set_key(&text, "X-GNOME-Autostart-enabled", "true");
    }
    std::fs::create_dir_all(&ud).map_err(fail)?;
    write(&user, &text).map_err(fail)
}

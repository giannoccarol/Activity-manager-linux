//! Persistent settings (~/.config/gestione-attivita/impostazioni.json),
//! KDE global shortcuts and autostart.
use crate::util::{config_home, run_status, which};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: String,
    pub interval_ms: u64,
    pub alerts: bool,
    pub tray: bool,
    pub autostart: bool,
    pub shortcuts: bool,
    /// "auto" (system language), "it" or "en"
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { theme: "graphite".into(), interval_ms: 1000, alerts: true, tray: true, autostart: false, shortcuts: true, language: "auto".into() }
    }
}

fn dir() -> PathBuf {
    config_home().join("gestione-attivita")
}

impl Settings {
    pub fn load() -> Self {
        std::fs::read_to_string(dir().join("impostazioni.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(dir());
        if let Ok(t) = serde_json::to_string_pretty(self) {
            let tmp = dir().join("impostazioni.json.tmp");
            if std::fs::write(&tmp, t).is_ok() {
                let _ = std::fs::rename(tmp, dir().join("impostazioni.json"));
            }
        }
    }
}

// ------------------------------------------------------------------ shortcuts (kglobalaccel)

const CAD: u32 = 218103815; // Ctrl+Alt+Del (Qt key code)
const CSE: u32 = 117440512; // Ctrl+Shift+Esc
const LOGOUT: [&str; 4] = ["ksmserver", "Log Out", "", ""];

/// kglobalaccel ids: [component, action, friendly component name, friendly action name] (names in the current language).
fn launch_id() -> [String; 4] {
    let n = crate::i18n::t("app.name");
    ["gestione-attivita.desktop".into(), "_launch".into(), n.clone(), n]
}

fn secure_id() -> [String; 4] {
    let n = crate::i18n::t("app.secureScreen");
    ["gestione-attivita-sicurezza.desktop".into(), "_launch".into(), n.clone(), n]
}

fn as_list<S: AsRef<str>>(v: &[S]) -> String {
    format!("[{}]", v.iter().map(|x| format!("'{}'", x.as_ref().replace('\'', "\\'"))).collect::<Vec<_>>().join(", "))
}

fn keys(k: &[u32]) -> String {
    if k.is_empty() {
        "@a(ai) []".into()
    } else {
        format!("[{}]", k.iter().map(|k| format!("([{k}, 0, 0, 0],)")).collect::<Vec<_>>().join(", "))
    }
}

fn kga(method: &str, args: &[String]) -> bool {
    let mut a: Vec<String> = ["call", "--session", "--dest", "org.kde.kglobalaccel", "--object-path", "/kglobalaccel", "--method"].iter().map(|s| s.to_string()).collect();
    a.push(format!("org.kde.KGlobalAccel.{method}"));
    a.extend(args.iter().cloned());
    let refs: Vec<&str> = a.iter().map(|s| s.as_str()).collect();
    run_status("gdbus", &refs, Duration::from_secs(10)).0
}

/// Ctrl+Alt+Del opens the security screen, Ctrl+Shift+Esc opens Gestione attività.
pub fn apply_shortcuts(on: bool) -> bool {
    if !which("gdbus") {
        return false;
    }
    if on {
        kga("setForeignShortcutKeys", &[as_list(&LOGOUT), keys(&[])]);
        let mut ok = true;
        for (id, k) in [(secure_id(), CAD), (launch_id(), CSE)] {
            kga("doRegister", &[as_list(&id)]);
            ok &= kga("setForeignShortcutKeys", &[as_list(&id), keys(&[k])]);
        }
        ok
    } else {
        for id in [secure_id(), launch_id()] {
            kga("setForeignShortcutKeys", &[as_list(&id), keys(&[])]);
        }
        kga("setForeignShortcutKeys", &[as_list(&LOGOUT), keys(&[CAD])])
    }
}

// ------------------------------------------------------------------ autostart

pub fn apply_autostart(on: bool, exe: &Path) {
    let f = config_home().join("autostart/gestione-attivita.desktop");
    if on {
        let _ = std::fs::create_dir_all(f.parent().unwrap_or(Path::new("/")));
        let body = format!(
            "[Desktop Entry]\nType=Application\nName=Activity Manager\nName[it]=Gestione attività\nComment=Stays in the system tray, ready for Ctrl+Alt+Del\nComment[it]=Resta nella barra di sistema, pronta per Ctrl+Alt+Canc\nExec={} --tray\nIcon=gestione-attivita\nX-GNOME-Autostart-enabled=true\nNoDisplay=true\n",
            exe.display()
        );
        let _ = std::fs::write(f, body);
    } else {
        let _ = std::fs::remove_file(f);
    }
}

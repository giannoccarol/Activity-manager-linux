//! Session actions for the Ctrl+Alt+Del screen: lock, switch user, log out, power.
use crate::util::run_status;
use std::time::Duration;

fn gdbus(bus: &str, dest: &str, path: &str, method: &str) -> bool {
    run_status("gdbus", &["call", bus, "--dest", dest, "--object-path", path, "--method", method], Duration::from_secs(8)).0
}

fn spawn(cmd: &str, args: &[&str]) {
    let _ = std::process::Command::new(cmd).args(args).spawn();
}

pub fn act(verb: &str) -> Result<(), String> {
    match verb {
        "lock" => {
            if !gdbus("--session", "org.freedesktop.ScreenSaver", "/ScreenSaver", "org.freedesktop.ScreenSaver.Lock") {
                spawn("loginctl", &["lock-session"]);
            }
        }
        "switch" => {
            if !gdbus("--system", "org.freedesktop.DisplayManager", "/org/freedesktop/DisplayManager/Seat0", "org.freedesktop.DisplayManager.Seat.SwitchToGreeter") {
                return act("lock");
            }
        }
        "logout" => {
            if !gdbus("--session", "org.kde.LogoutPrompt", "/LogoutPrompt", "org.kde.LogoutPrompt.promptLogout") {
                gdbus("--session", "org.kde.Shutdown", "/Shutdown", "org.kde.Shutdown.logout");
            }
        }
        "shutdown" => {
            if !gdbus("--session", "org.kde.LogoutPrompt", "/LogoutPrompt", "org.kde.LogoutPrompt.promptShutDown") {
                spawn("systemctl", &["poweroff"]);
            }
        }
        "reboot" => {
            if !gdbus("--session", "org.kde.LogoutPrompt", "/LogoutPrompt", "org.kde.LogoutPrompt.promptReboot") {
                spawn("systemctl", &["reboot"]);
            }
        }
        "suspend" => spawn("systemctl", &["suspend"]),
        _ => return Err(crate::i18n::tf("session.unknownAction", &[("action", verb.to_string())])),
    }
    Ok(())
}

/// Full name, username, avatar and computer name for the security screen.
pub fn info() -> serde_json::Value {
    let uid = unsafe { libc::getuid() };
    let pw = crate::util::read_raw("/etc/passwd").unwrap_or_default();
    let (user, full) = pw
        .lines()
        .map(|l| l.split(':').collect::<Vec<_>>())
        .find(|f| f.len() > 4 && f[2].parse::<u32>().ok() == Some(uid))
        .map(|f| (f[0].to_string(), f[4].split(',').next().unwrap_or("").to_string()))
        .unwrap_or_default();
    let home = crate::util::home();
    let avatar = [home.join(".face.icon"), home.join(".face"), format!("/var/lib/AccountsService/icons/{user}").into()]
        .into_iter()
        .find(|p| p.is_file())
        .map(|p| std::fs::canonicalize(&p).unwrap_or(p).to_string_lossy().into_owned());
    let host = crate::util::read("/proc/sys/kernel/hostname").unwrap_or_default();
    serde_json::json!({ "user": user, "fullName": if full.is_empty() { user.clone() } else { full }, "avatar": avatar, "host": host })
}

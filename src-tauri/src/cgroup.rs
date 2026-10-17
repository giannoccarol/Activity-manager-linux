//! Interpretazione dei cgroup di systemd: a quale app, servizio o container appartiene un processo.
use crate::util::unescape;

pub struct CgInfo {
    pub unit_path: String,
    pub unit: Option<String>,
    pub user_manager: Option<String>,
}

pub fn parse(path: &str) -> CgInfo {
    if path.is_empty() || path == "/" {
        return CgInfo { unit_path: "/".into(), unit: None, user_manager: None };
    }
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let mut uid = None;
    for (i, p) in parts.iter().enumerate() {
        if p.starts_with("user@") && p.ends_with(".service") {
            uid = Some(p[5..p.len() - 8].to_string());
            continue;
        }
        if p.ends_with(".slice") {
            continue;
        }
        if p.ends_with(".service") || p.ends_with(".scope") {
            return CgInfo { unit_path: format!("/{}", parts[..=i].join("/")), unit: Some(p.to_string()), user_manager: uid };
        }
    }
    if let Some(u) = &uid {
        let name = format!("user@{u}.service");
        if let Some(i) = parts.iter().position(|p| *p == name) {
            return CgInfo { unit_path: format!("/{}", parts[..=i].join("/")), unit: Some(name), user_manager: uid };
        }
    }
    CgInfo { unit_path: format!("/{}", parts.join("/")), unit: None, user_manager: uid }
}

/// Unità di un'app avviata dall'utente: app-<id>-<rnd>.scope oppure app-<id>@<rnd>.service.
pub fn is_app_unit(unit: &str) -> bool {
    if !unit.starts_with("app-") {
        return false;
    }
    if unit.ends_with(".scope") {
        return true;
    }
    if let Some(stem) = unit.strip_suffix(".service") {
        if let Some((_, inst)) = stem.rsplit_once('@') {
            return inst != "autostart" && !unit.starts_with("app-dbus");
        }
    }
    false
}

pub fn is_container(unit: &str) -> bool {
    unit.ends_with(".scope") && (unit.starts_with("docker-") || unit.starts_with("libpod-"))
}

pub fn appid(unit: &str) -> Option<String> {
    let mut u = unescape(unit);
    if u.starts_with("dbus-:") && u.ends_with(".service") {
        u = format!("app-{u}");
    }
    let body = u.strip_prefix("app-")?;
    let mut body = if let Some(b) = body.strip_suffix(".scope") {
        b.rsplit_once('-').map(|(a, _)| a.to_string()).unwrap_or_else(|| b.to_string())
    } else if let Some(b) = body.strip_suffix(".service") {
        b.split('@').next().unwrap_or(b).to_string()
    } else {
        body.to_string()
    };
    if body.starts_with("dbus-:") {
        let parts: Vec<&str> = body.splitn(3, '-').collect();
        if parts.len() == 3 {
            body = parts[2].to_string();
        }
    }
    if body.starts_with("gestione-attivita") {
        body = "gestione-attivita".into();
    }
    (!body.is_empty()).then_some(body)
}

//! Index of .desktop files (localized names, icons) and icon resolution from the theme.
use crate::util::{data_home, home, read_raw};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub icon: Option<String>,
    pub exec_base: Option<String>,
    pub nodisplay: bool,
}

#[derive(Default)]
pub struct DesktopDb {
    by_id: HashMap<String, Entry>,
    by_id_lower: HashMap<String, Entry>,
    by_short: HashMap<String, Entry>,
    by_exec: HashMap<String, Entry>,
    by_wmclass: HashMap<String, Entry>,
}

/// Name[xx] variants to look for, in order, for the interface language: Italian prefers Name[it_IT] and Name[it];
/// English uses the system's English variant if there is one (Name[en_GB]), otherwise the plain Name.
fn langs() -> Vec<String> {
    let raw = ["LC_ALL", "LC_MESSAGES", "LANG"].iter().find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty())).unwrap_or_default();
    let base = raw.split(['.', '@']).next().unwrap_or("").to_string();
    let lang = crate::i18n::lang();
    let mut v = vec![];
    if base.split('_').next() == Some(lang) && base.contains('_') {
        v.push(base);
    } else if lang == "it" {
        v.push("it_IT".into());
    }
    if lang == "it" {
        v.push("it".into());
    }
    v
}

pub fn parse_desktop(path: &Path) -> Option<HashMap<String, String>> {
    let txt = read_raw(path)?;
    let mut raw: HashMap<String, String> = HashMap::new();
    let mut in_entry = false;
    for line in txt.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_entry = l == "[Desktop Entry]";
            continue;
        }
        if !in_entry || l.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = l.split_once('=') {
            raw.entry(k.trim().to_string()).or_insert_with(|| v.trim().to_string());
        }
    }
    let ls = langs();
    let mut out = HashMap::new();
    for (k, v) in raw.iter() {
        if k.contains('[') {
            continue;
        }
        let mut val = v.clone();
        for l in &ls {
            if let Some(lv) = raw.get(&format!("{k}[{l}]")) {
                val = lv.clone();
                break;
            }
        }
        out.insert(k.clone(), val);
    }
    Some(out)
}

pub fn exec_base(exec: &str) -> Option<String> {
    let toks: Vec<&str> = exec.split_whitespace().map(|t| t.trim_matches(|c| c == '"' || c == '\'')).collect();
    let mut i = 0;
    while i < toks.len() {
        let t = toks[i];
        if t == "env" || (t.contains('=') && !t.starts_with('/') && !t.starts_with('-')) {
            i += 1;
            continue;
        }
        if t.ends_with("flatpak") {
            if let Some(p) = toks.iter().position(|x| *x == "run") {
                return toks[p + 1..].iter().find(|x| !x.starts_with('-') && !x.starts_with('@')).map(|s| s.to_string());
            }
        }
        return Path::new(t).file_name().map(|s| s.to_string_lossy().into_owned());
    }
    None
}

pub fn data_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![data_home()];
    let sys = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    dirs.extend(sys.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));
    dirs.push(home().join(".local/share/flatpak/exports/share"));
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share"));
    let mut seen = std::collections::HashSet::new();
    dirs.into_iter().filter(|d| d.is_dir() && seen.insert(d.clone())).collect()
}

impl DesktopDb {
    pub fn load() -> Self {
        let mut db = DesktopDb::default();
        for d in data_dirs() {
            let root = d.join("applications");
            let mut stack = vec![root.clone()];
            while let Some(dir) = stack.pop() {
                let Ok(rd) = std::fs::read_dir(&dir) else { continue };
                for e in rd.flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        stack.push(p);
                        continue;
                    }
                    if p.extension().map(|x| x != "desktop").unwrap_or(true) {
                        continue;
                    }
                    let rel = p.strip_prefix(&root).unwrap_or(&p).to_string_lossy().replace('/', "-");
                    let id = rel.trim_end_matches(".desktop").to_string();
                    if db.by_id.contains_key(&id) {
                        continue;
                    }
                    let Some(kv) = parse_desktop(&p) else { continue };
                    if kv.get("Type").map(|t| t != "Application").unwrap_or(false) {
                        continue;
                    }
                    let entry = Entry {
                        name: kv.get("Name").cloned().unwrap_or_else(|| id.clone()),
                        icon: kv.get("Icon").cloned(),
                        exec_base: kv.get("Exec").or(kv.get("TryExec")).and_then(|e| exec_base(e)),
                        nodisplay: kv.get("NoDisplay").map(|v| v.eq_ignore_ascii_case("true")).unwrap_or(false),
                    };
                    db.by_id_lower.entry(id.to_lowercase()).or_insert_with(|| entry.clone());
                    db.by_short
                        .entry(id.rsplit('.').next().unwrap_or(&id).to_lowercase())
                        .or_insert_with(|| entry.clone());
                    if let Some(eb) = &entry.exec_base {
                        let replace = db.by_exec.get(eb).map(|c| c.nodisplay && !entry.nodisplay).unwrap_or(true);
                        if replace {
                            db.by_exec.insert(eb.clone(), entry.clone());
                        }
                    }
                    if let Some(wm) = kv.get("StartupWMClass") {
                        db.by_wmclass.entry(wm.to_lowercase()).or_insert_with(|| entry.clone());
                    }
                    db.by_id.insert(id, entry);
                }
            }
        }
        db
    }

    pub fn lookup_appid(&self, appid: &str) -> Option<&Entry> {
        let mut cands = vec![appid.to_string()];
        for pre in ["flatpak-", "snap-", "gnome-", "kde-", "xdg-"] {
            if let Some(r) = appid.strip_prefix(pre) {
                cands.push(r.to_string());
            }
        }
        for c in &cands {
            if let Some(e) = self.by_id.get(c).or_else(|| self.by_id_lower.get(&c.to_lowercase())) {
                return Some(e);
            }
        }
        for c in &cands {
            if let Some(e) = self.by_wmclass.get(&c.to_lowercase()).or_else(|| self.by_exec.get(c)) {
                return Some(e);
            }
        }
        None
    }

    pub fn lookup_exec(&self, name: &str) -> Option<&Entry> {
        if name.is_empty() {
            return None;
        }
        let l = name.to_lowercase();
        self.by_exec
            .get(name)
            .or_else(|| self.by_wmclass.get(&l))
            .or_else(|| self.by_id_lower.get(&l))
            .or_else(|| self.by_short.get(&l))
    }
}

/// Resolves icon names to files (KDE theme, hicolor, pixmaps), with a cache.
pub struct Icons {
    themes: Vec<PathBuf>,
    cache: HashMap<String, Option<String>>,
}

const CONTEXTS: [&str; 7] = ["apps", "preferences", "devices", "categories", "places", "status", "actions"];
const SIZES: [&str; 9] = ["scalable", "256x256", "128x128", "96x96", "64x64", "48x48", "512x512", "32x32", "symbolic"];

impl Icons {
    pub fn new(dark: bool) -> Self {
        let kde = read_raw(crate::util::config_home().join("kdeglobals"))
            .and_then(|t| {
                let mut in_icons = false;
                for l in t.lines() {
                    let l = l.trim();
                    if l.starts_with('[') {
                        in_icons = l == "[Icons]";
                    } else if in_icons {
                        if let Some(v) = l.strip_prefix("Theme=") {
                            return Some(v.to_string());
                        }
                    }
                }
                None
            })
            .unwrap_or_else(|| "breeze".into());
        let mut names = vec![];
        if dark && !kde.ends_with("-dark") {
            names.push(format!("{kde}-dark"));
        }
        names.push(kde);
        names.push("breeze".into());
        names.push("hicolor".into());
        let mut bases = vec![home().join(".local/share/icons"), home().join(".icons")];
        for d in data_dirs() {
            bases.push(d.join("icons"));
        }
        let mut themes = vec![];
        for n in names {
            for b in &bases {
                let p = b.join(&n);
                if p.is_dir() && !themes.contains(&p) {
                    themes.push(p);
                }
            }
        }
        Icons { themes, cache: HashMap::new() }
    }

    pub fn resolve(&mut self, name: &str) -> Option<String> {
        if name.is_empty() {
            return None;
        }
        if let Some(v) = self.cache.get(name) {
            return v.clone();
        }
        // real path: Tauri's asset protocol rejects relative symlinks (common in Breeze)
        let found = self.find(name).map(|p| std::fs::canonicalize(&p).unwrap_or(p).to_string_lossy().into_owned());
        self.cache.insert(name.to_string(), found.clone());
        found
    }

    fn find(&self, name: &str) -> Option<PathBuf> {
        let p = Path::new(name);
        if p.is_absolute() {
            return p.is_file().then(|| p.to_path_buf());
        }
        for ctx in CONTEXTS {
            for t in &self.themes {
                // hicolor layout: <theme>/<size>/<context>/name
                for s in SIZES {
                    for ext in ["svg", "png"] {
                        let f = t.join(s).join(ctx).join(format!("{name}.{ext}"));
                        if f.is_file() {
                            return Some(f);
                        }
                    }
                }
                // Breeze layout: <theme>/<context>/<size>/name
                for s in ["64", "48", "32", "24", "22", "16"] {
                    let f = t.join(ctx).join(s).join(format!("{name}.svg"));
                    if f.is_file() {
                        return Some(f);
                    }
                }
            }
            if ctx == "apps" {
                for ext in ["png", "svg"] {
                    let f = PathBuf::from(format!("/usr/share/pixmaps/{name}.{ext}"));
                    if f.is_file() {
                        return Some(f);
                    }
                }
            }
        }
        None
    }
}

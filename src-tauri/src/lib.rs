//! Gestione attività — Tauri backend.
mod actions;
mod blur;
mod cgroup;
mod collect;
mod desktop;
mod extra;
mod fmt;
mod i18n;
mod insights;
mod model;
mod session;
mod settings;
mod util;

use settings::Settings;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};

struct AppState {
    last: actions::Last,
    started: AtomicBool,
    interval_ms: Arc<AtomicU64>,
    settings: Arc<Mutex<Settings>>,
    history: Arc<Mutex<extra::history::History>>,
    netapps: Mutex<extra::netapp::NetApps>,
    /// Started only for the Ctrl+Alt+Del screen: exits on close, unless it stays in the tray.
    secure_only: AtomicBool,
    /// The UI was (re)loaded: everything must be resent, static fields included.
    resync: Arc<AtomicBool>,
}

// ------------------------------------------------------------------ windows

/// Window corner radius (must match --win-r in app.css).
const WIN_R: i32 = 12;

/// Real KWin blur behind a window (Glass theme and security screen).
fn glass(app: &AppHandle, label: &str, on: bool) {
    let Some(w) = app.get_webview_window(label) else { return };
    let label = label.to_string();
    let radius = if label == "main" { WIN_R } else { 0 };
    let _ = app.run_on_main_thread(move || {
        if let Ok(gw) = w.gtk_window() {
            blur::apply(&label, &gw, on, radius);
        }
    });
}

fn main_glass_on(app: &AppHandle) -> bool {
    app.state::<AppState>().settings.lock().map(|s| s.theme == "glass").unwrap_or(false)
}

/// Hands GTK the Wayland activation token received from the shortcut: without it, KWin
/// will not raise (or unminimize) an already existing window.
fn present(app: &AppHandle, label: &str, token: Option<String>) {
    let Some(w) = app.get_webview_window(label) else { return };
    let _ = app.run_on_main_thread(move || {
        use gtk::prelude::*;
        if let Ok(gw) = w.gtk_window() {
            if let Some(t) = token.as_deref() {
                gw.set_startup_id(t);
            }
            gw.present();
        }
    });
}

fn token_of(argv: &[String]) -> Option<String> {
    argv.iter().find_map(|a| a.strip_prefix("--activation-token=")).map(String::from)
}

fn show_main(app: &AppHandle) {
    show_main_with(app, None);
}

fn show_main_with(app: &AppHandle, token: Option<String>) {
    let st = app.state::<AppState>();
    st.secure_only.store(false, Ordering::SeqCst);
    start_collector(app, &st);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
    present(app, "main", token);
    if main_glass_on(app) {
        glass(app, "main", true);
    }
}

fn close_secure(app: &AppHandle) {
    for (label, w) in app.webview_windows() {
        if label.starts_with("secure-") {
            let _ = w.destroy();
        }
    }
}

/// Full-screen Ctrl+Alt+Del screen on every monitor; the buttons are on the primary one.
fn open_secure(app: &AppHandle) {
    open_secure_with(app, None);
}

fn open_secure_with(app: &AppHandle, token: Option<String>) {
    if let Some((label, _)) = app.webview_windows().into_iter().find(|(l, _)| l.starts_with("secure-")) {
        present(app, &label, token);
        return;
    }
    let monitors = app.available_monitors().unwrap_or_default();
    let primary = app.primary_monitor().ok().flatten().map(|m| m.name().cloned());
    let n = monitors.len().max(1);
    let main_idx = monitors.iter().position(|m| Some(m.name().cloned()) == primary).unwrap_or(0);
    for i in 0..n {
        let is_main = i == main_idx;
        let script = format!("window.__VIEW__ = 'secure'; window.__PRIMARY__ = {is_main};");
        let built = WebviewWindowBuilder::new(app, format!("secure-{i}"), WebviewUrl::default())
            .title(i18n::t("window.security"))
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .visible(false)
            .transparent(true)
            .initialization_script(&script)
            .build();
        let Ok(w) = built else { continue };
        let w2 = w.clone();
        let token = if is_main { token.clone() } else { None };
        let _ = app.run_on_main_thread(move || {
            use gtk::prelude::*;
            if let Ok(gw) = w2.gtk_window() {
                if let Some(t) = token.as_deref() {
                    gw.set_startup_id(t);
                }
                let screen = WidgetExt::screen(&gw).or_else(gtk::gdk::Screen::default);
                if let Some(screen) = screen {
                    gw.fullscreen_on_monitor(&screen, i as i32);
                } else {
                    gw.fullscreen();
                }
            }
            let _ = w2.show();
            if is_main {
                let _ = w2.set_focus();
                if let Ok(gw) = w2.gtk_window() {
                    gw.present();
                }
            }
            if let Ok(gw) = w2.gtk_window() {
                blur::apply(w2.label(), &gw, true, 0);
            }
        });
    }
}

fn after_secure(app: &AppHandle) {
    close_secure(app);
    let st = app.state::<AppState>();
    let tray = st.settings.lock().map(|s| s.tray).unwrap_or(false);
    if st.secure_only.load(Ordering::SeqCst) && !tray {
        app.exit(0);
    }
}

/// Captures the contents of an app window as PNG (diagnostics: `gestione-attivita --snapshot=/path.png[:label]`).
fn snapshot_window(app: &AppHandle, arg: &str) {
    let (path, label) = arg.split_once(':').map(|(p, l)| (p.to_string(), l.to_string())).unwrap_or((arg.to_string(), "main".into()));
    let Some(w) = app.get_webview_window(&label) else { return };
    let _ = w.with_webview(move |wv| {
        use webkit2gtk::{SnapshotOptions, SnapshotRegion, WebViewExt};
        let view = wv.inner();
        view.snapshot(SnapshotRegion::Visible, SnapshotOptions::NONE, None::<&gtk::gio::Cancellable>, move |r| {
            if let Ok(surface) = r {
                if let Ok(mut f) = std::fs::File::create(&path) {
                    let _ = surface.write_to_png(&mut f);
                }
            }
        });
    });
}

// ------------------------------------------------------------------ system tray

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    if app.tray_by_id("main").is_some() {
        return Ok(());
    }
    let open = MenuItem::with_id(app, "open", i18n::t("tray.open"), true, None::<&str>)?;
    let secure = MenuItem::with_id(app, "secure", i18n::t("tray.secure"), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", i18n::t("tray.quit"), true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &secure, &PredefinedMenuItem::separator(app)?, &quit])?;
    let mut b = TrayIconBuilder::with_id("main").tooltip(i18n::t("app.name")).menu(&menu).on_menu_event(|app, e| match e.id().as_ref() {
        "open" => show_main(app),
        "secure" => open_secure(app),
        "quit" => app.exit(0),
        _ => {}
    });
    if let Some(icon) = app.default_window_icon() {
        b = b.icon(icon.clone());
    }
    b.build(app)?;
    Ok(())
}

// ------------------------------------------------------------------ collection and alerts

/// Alerts snoozed by the user with «Ignora per 2 ore» ("ignore for 2 hours"), per resource.
fn snoozed() -> &'static Mutex<HashMap<String, Instant>> {
    static S: std::sync::OnceLock<Mutex<HashMap<String, Instant>>> = std::sync::OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

fn notify(app: AppHandle, res: String, title: String, body: String) {
    let name = i18n::t("app.name");
    let show = format!("--action=open={}", i18n::t("notify.show"));
    let snooze = format!("--action=snooze={}", i18n::t("notify.snooze"));
    std::thread::spawn(move || {
        let out = std::process::Command::new("notify-send")
            .args(["-a", &name, "-i", "gestione-attivita", "-h", "string:desktop-entry:gestione-attivita"])
            .args([&show, &snooze, "-w", &title, &body])
            .output();
        let Ok(o) = out else { return };
        match String::from_utf8_lossy(&o.stdout).trim() {
            "open" => {
                let a = app.clone();
                let _ = app.run_on_main_thread(move || show_main(&a));
            }
            "snooze" => {
                if let Ok(mut m) = snoozed().lock() {
                    m.insert(res, Instant::now() + Duration::from_secs(7200));
                }
            }
            _ => {}
        }
    });
}

fn start_collector(app: &AppHandle, st: &AppState) {
    if st.started.swap(true, Ordering::SeqCst) {
        return;
    }
    let (app, last, iv, settings, history, resync) = (app.clone(), st.last.clone(), st.interval_ms.clone(), st.settings.clone(), st.history.clone(), st.resync.clone());
    std::thread::Builder::new()
        .name("collector".into())
        .spawn(move || {
            let mut c = collect::Collector::new();
            let mut sent: HashMap<String, Instant> = HashMap::new();
            let mut prev = Instant::now();
            let mut wired: std::collections::HashSet<(i32, u64)> = std::collections::HashSet::new();
            loop {
                let t0 = Instant::now();
                let snap = c.collect();
                let dt = t0.duration_since(prev).as_secs_f64();
                prev = t0;
                if let Ok(mut h) = history.lock() {
                    h.record(&snap, dt);
                }
                let win = app.get_webview_window("main");
                // minimized or hidden: nothing is sent, so the UI does not waste CPU redrawing
                let visible = win.as_ref().map(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false)).unwrap_or(false);
                let focused = visible && win.as_ref().map(|w| w.is_focused().unwrap_or(false)).unwrap_or(false);
                if visible {
                    if resync.swap(false, Ordering::SeqCst) {
                        wired.clear();
                    }
                    if let Some(w) = &win {
                        let _ = w.emit("snapshot", snap.wire(&mut wired));
                    }
                }
                let alerts = settings.lock().map(|s| s.alerts).unwrap_or(false);
                if alerts && !focused {
                    for ins in snap.insights.iter().filter(|i| i.level == "critical") {
                        let due = sent.get(&ins.res).map(|t| t.elapsed() > Duration::from_secs(1800)).unwrap_or(true);
                        let quiet = snoozed().lock().ok().and_then(|m| m.get(&ins.res).copied()).map(|t| t > Instant::now()).unwrap_or(false);
                        if due && !quiet {
                            sent.insert(ins.res.clone(), Instant::now());
                            notify(app.clone(), ins.res.clone(), ins.title.clone(), ins.sentence.clone());
                        }
                    }
                }
                if let Ok(mut l) = last.lock() {
                    *l = Some(snap);
                }
                // window not visible: one sample every 3 s is enough for history and alerts
                let ms = iv.load(Ordering::Relaxed).max(250);
                let ms = if visible { ms } else { ms.max(3000) };
                let deadline = t0 + Duration::from_millis(ms);
                // if the window becomes visible again, the normal pace resumes immediately
                while Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(100).min(deadline.saturating_duration_since(Instant::now())));
                    if !visible && app.get_webview_window("main").map(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false)).unwrap_or(false) {
                        break;
                    }
                }
            }
        })
        .expect("collector thread");
}

// ------------------------------------------------------------------ commands

#[tauri::command]
fn start_monitor(app: AppHandle, state: State<AppState>) {
    // the UI has just been loaded: it will receive everything in full on the next round
    state.resync.store(true, Ordering::SeqCst);
    start_collector(&app, &state);
    if let Some(s) = state.last.lock().ok().and_then(|l| l.clone()) {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.emit("snapshot", s.wire(&mut Default::default()));
        }
    }
}

/// `--bench` results: in ~/.cache/gestione-attivita/bench.txt, with the size of the data sent.
#[tauri::command]
fn bench_report(state: State<AppState>, text: String) {
    let (full, steady) = state
        .last
        .lock()
        .ok()
        .and_then(|l| {
            l.as_ref().map(|s| {
                let mut sent = std::collections::HashSet::new();
                let first = serde_json::to_vec(&s.wire(&mut sent)).map(|v| v.len()).unwrap_or(0);
                let next = serde_json::to_vec(&s.wire(&mut sent)).map(|v| v.len()).unwrap_or(0);
                (first, next)
            })
        })
        .unwrap_or((0, 0));
    let dir = util::home().join(".cache/gestione-attivita");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("bench.txt"), format!("{text}\nData sent: {} KB on the first update, then {} KB per update\n", full / 1024, steady / 1024));
}

/// CPU ticks accumulated by the app (process + WebKit children) and KWin, with the timestamp: for `--bench`.
#[tauri::command]
fn bench_cpu() -> (f64, f64, f64) {
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
    let ticks = |pid: u32| -> f64 {
        util::read_raw(format!("/proc/{pid}/stat"))
            .and_then(|s| {
                let f: Vec<&str> = s[s.rfind(')')? + 2..].split_whitespace().collect();
                Some(f.get(11)?.parse::<f64>().ok()? + f.get(12)?.parse::<f64>().ok()?)
            })
            .unwrap_or(0.0)
    };
    let me = std::process::id();
    let mut app = ticks(me);
    let mut kwin = 0.0;
    if let Ok(rd) = std::fs::read_dir("/proc") {
        for e in rd.flatten() {
            let Some(pid) = e.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else { continue };
            let Some(st) = util::read_raw(format!("/proc/{pid}/stat")) else { continue };
            let comm = &st[st.find('(').map(|i| i + 1).unwrap_or(0)..st.rfind(')').unwrap_or(0)];
            let ppid: u32 = st[st.rfind(')').unwrap_or(0) + 2..].split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
            if ppid == me && comm.starts_with("WebKit") {
                app += ticks(pid);
            } else if comm == "kwin_wayland" {
                kwin += ticks(pid);
            }
        }
    }
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
    (app / hz, kwin / hz, t)
}

#[tauri::command]
fn log(msg: String) {
    eprintln!("[ui] {msg}");
}

#[tauri::command]
async fn action(state: State<'_, AppState>, action: String, payload: serde_json::Value) -> Result<String, String> {
    actions::handle(&state.last, &action, &payload)
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().map(|s| s.clone()).unwrap_or_default()
}

#[tauri::command]
async fn set_settings(app: AppHandle, state: State<'_, AppState>, patch: serde_json::Value) -> Result<Settings, String> {
    let (old, new) = {
        let mut cur = state.settings.lock().map_err(|_| i18n::t("settings.unavailable"))?;
        let old = cur.clone();
        let mut v = serde_json::to_value(&*cur).map_err(|e| e.to_string())?;
        if let (Some(o), Some(p)) = (v.as_object_mut(), patch.as_object()) {
            for (k, val) in p {
                o.insert(k.clone(), val.clone());
            }
        }
        *cur = serde_json::from_value(v).map_err(|e| e.to_string())?;
        cur.save();
        (old, cur.clone())
    };
    state.interval_ms.store(new.interval_ms, Ordering::Relaxed);
    if old.shortcuts != new.shortcuts && !settings::apply_shortcuts(new.shortcuts) {
        return Err(i18n::t("settings.shortcutsRejected"));
    }
    if old.autostart != new.autostart {
        let exe = std::env::current_exe().unwrap_or_default();
        settings::apply_autostart(new.autostart, &exe);
    }
    if old.language != new.language {
        i18n::set(&new.language);
        // the tray menu is built with the labels of the current language
        if app.tray_by_id("main").is_some() {
            let _ = app.remove_tray_by_id("main");
            build_tray(&app).map_err(|e| e.to_string())?;
        }
    }
    if old.theme != new.theme {
        glass(&app, "main", new.theme == "glass");
    }
    if old.tray != new.tray {
        if new.tray {
            build_tray(&app).map_err(|e| e.to_string())?;
        } else {
            let _ = app.remove_tray_by_id("main");
        }
    }
    Ok(new)
}

/// System language ("it" or "en"), used by the interface when the language is "auto".
#[tauri::command]
fn system_lang() -> &'static str {
    i18n::system_lang()
}

#[tauri::command]
fn session_info() -> serde_json::Value {
    session::info()
}

#[tauri::command]
async fn session(app: AppHandle, verb: String) -> Result<(), String> {
    match verb.as_str() {
        "cancel" => {
            after_secure(&app);
            Ok(())
        }
        "taskmgr" => {
            close_secure(&app);
            show_main(&app);
            Ok(())
        }
        _ => {
            close_secure(&app);
            std::thread::sleep(Duration::from_millis(120));
            let r = session::act(&verb);
            after_secure(&app);
            r
        }
    }
}

#[tauri::command]
async fn history(state: State<'_, AppState>, days: u32) -> Result<Vec<extra::history::HistoryRow>, String> {
    Ok(state.history.lock().map_err(|_| i18n::t("history.unavailable"))?.query(days.max(1)))
}

#[tauri::command]
async fn history_clear(state: State<'_, AppState>) -> Result<(), String> {
    state.history.lock().map_err(|_| i18n::t("history.unavailable"))?.clear();
    Ok(())
}

#[tauri::command]
async fn net_apps(state: State<'_, AppState>) -> Result<Vec<extra::netapp::NetApp>, String> {
    let snap = state.last.lock().map_err(|_| i18n::t("network.dataUnavailable"))?.clone();
    let Some(snap) = snap else { return Ok(vec![]) };
    Ok(state.netapps.lock().map_err(|_| i18n::t("network.unavailable"))?.sample(&snap))
}

#[tauri::command]
async fn startup_list() -> Vec<extra::startup::StartupItem> {
    extra::startup::list()
}

#[tauri::command]
async fn startup_set(id: String, enabled: bool) -> Result<(), String> {
    extra::startup::set_enabled(&id, enabled)
}

#[tauri::command]
async fn services_list() -> Vec<extra::services::Service> {
    extra::services::list()
}

#[tauri::command]
async fn service_action(unit: String, scope: String, verb: String) -> Result<String, String> {
    extra::services::action(&unit, &scope, &verb)
}

/// Assigns (or removes, restoring Log Out on Ctrl+Alt+Del) the KDE shortcuts. Used by the installer.
pub fn shortcuts(on: bool) -> bool {
    // the shortcuts are registered with names in the interface language
    i18n::set(&Settings::load().language);
    settings::apply_shortcuts(on)
}

/// Prints a snapshot as JSON (diagnostics: `gestione-attivita --dump`), in the language of the settings.
pub fn dump() {
    i18n::set(&Settings::load().language);
    let mut c = collect::Collector::new();
    c.collect();
    std::thread::sleep(Duration::from_millis(1000));
    let s = c.collect();
    println!("{}", serde_json::to_string_pretty(&s).unwrap_or_default());
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    let secure = args.iter().any(|a| a == "--secure");
    let tray_only = args.iter().any(|a| a == "--tray");
    let cfg = Settings::load();
    i18n::set(&cfg.language);
    let state = AppState {
        last: Arc::new(Mutex::new(None)),
        started: AtomicBool::new(false),
        interval_ms: Arc::new(AtomicU64::new(cfg.interval_ms)),
        settings: Arc::new(Mutex::new(cfg)),
        history: Arc::new(Mutex::new(extra::history::History::open())),
        netapps: Mutex::new(extra::netapp::NetApps::new()),
        secure_only: AtomicBool::new(secure),
        resync: Arc::new(AtomicBool::new(true)),
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(a) = argv.iter().find_map(|a| a.strip_prefix("--snapshot=")) {
                snapshot_window(app, a);
            } else if argv.iter().any(|a| a == "--bench") {
                show_main(app);
                let a = app.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(1500));
                    let _ = a.emit("bench", ());
                });
            } else if argv.iter().any(|a| a == "--secure-close") {
                after_secure(app);
            } else if let Some(p) = argv.iter().find_map(|a| a.strip_prefix("--page=")) {
                let _ = app.emit("navigate", p.to_string());
            } else if argv.iter().any(|a| a == "--secure") {
                open_secure_with(app, token_of(&argv));
            } else if !argv.iter().any(|a| a == "--tray") {
                show_main_with(app, token_of(&argv));
            }
        }))
        .manage(state)
        .setup(move |app| {
            let h = app.handle().clone();
            let st = h.state::<AppState>();
            let tray = st.settings.lock().map(|s| s.tray).unwrap_or(false);
            if tray || tray_only {
                if let Err(e) = build_tray(&h) {
                    eprintln!("system tray unavailable: {e}");
                }
            }
            if secure {
                open_secure(&h);
            } else if tray_only {
                start_collector(&h, &st);
            } else {
                show_main(&h);
            }
            Ok(())
        })
        .on_window_event(|w, e| {
            if w.label() != "main" {
                return;
            }
            if let WindowEvent::Resized(_) = e {
                let app = w.app_handle();
                if main_glass_on(app) {
                    glass(app, "main", true);
                }
            }
            if let WindowEvent::CloseRequested { api, .. } = e {
                let app = w.app_handle();
                let tray = app.state::<AppState>().settings.lock().map(|s| s.tray).unwrap_or(false);
                if tray && app.tray_by_id("main").is_some() {
                    api.prevent_close();
                    let _ = w.hide();
                } else {
                    app.exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            start_monitor,
            log,
            bench_report,
            bench_cpu,
            action,
            get_settings,
            set_settings,
            session_info,
            system_lang,
            session,
            history,
            history_clear,
            net_apps,
            startup_list,
            startup_set,
            services_list,
            service_action
        ])
        .build(tauri::generate_context!())
        .expect("failed to start Gestione attività")
        .run(|app, e| {
            // on exit, history still in memory must be written to disk
            if let tauri::RunEvent::Exit = e {
                if let Ok(mut h) = app.state::<AppState>().history.lock() {
                    *h = extra::history::History::open();
                }
            }
        });
}

//! Cronologia app: consumo accumulato per app e per giorno in SQLite (stesso database della versione Python).
use crate::desktop::Icons;
use crate::model::{Section, Snapshot};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRow {
    pub app: String,
    pub kind: String,
    pub icon: Option<String>,
    pub cpu_s: f64,
    pub disk_b: f64,
    pub gpu_s: f64,
    pub mem_peak: f64,
    pub last: f64,
}

#[derive(Default)]
struct Acc {
    kind: String,
    icon: String,
    cpu_s: f64,
    disk_b: f64,
    gpu_s: f64,
    mem_peak: f64,
    last: f64,
}

pub struct History {
    conn: Option<Connection>,
    acc: HashMap<(String, String), Acc>,
    last_flush: Instant,
    icons: Option<Icons>,
}

const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS usage (
    day TEXT, app TEXT, kind TEXT, icon TEXT,
    cpu_s REAL DEFAULT 0, disk_b REAL DEFAULT 0, gpu_s REAL DEFAULT 0, mem_peak REAL DEFAULT 0,
    last REAL DEFAULT 0, PRIMARY KEY (day, app))";

const UPSERT: &str = "INSERT INTO usage (day, app, kind, icon, cpu_s, disk_b, gpu_s, mem_peak, last)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
    ON CONFLICT(day, app) DO UPDATE SET kind = excluded.kind, icon = COALESCE(NULLIF(excluded.icon, ''), icon),
        cpu_s = cpu_s + excluded.cpu_s, disk_b = disk_b + excluded.disk_b,
        gpu_s = gpu_s + excluded.gpu_s, mem_peak = MAX(mem_peak, excluded.mem_peak),
        last = MAX(last, excluded.last)";

/// Data locale (AAAA-MM-GG) di `back` giorni fa, come `datetime.date.today()` in Python.
fn day(back: i64) -> String {
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut tm);
        tm.tm_mday -= back as libc::c_int;
        tm.tm_hour = 12;
        tm.tm_isdst = -1;
        libc::mktime(&mut tm);
        format!("{:04}-{:02}-{:02}", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday)
    }
}

fn kind(s: Section) -> &'static str {
    match s {
        Section::App => "app",
        Section::Ctr => "ctr",
        Section::Bg => "bg",
        Section::Sys => "sys",
    }
}

fn open_db() -> Option<Connection> {
    let dir = crate::util::data_home().join("gestione-attivita");
    std::fs::create_dir_all(&dir).ok()?;
    let conn = Connection::open(dir.join("history.sqlite")).ok()?;
    let _ = conn.busy_timeout(Duration::from_secs(2));
    conn.execute_batch(SCHEMA).ok()?;
    Some(conn)
}

impl History {
    pub fn open() -> Self {
        History { conn: open_db(), acc: HashMap::new(), last_flush: Instant::now(), icons: None }
    }

    /// Chiamata a ogni campionamento (circa 1 s) dal thread di raccolta.
    pub fn record(&mut self, snap: &Snapshot, dt: f64) {
        if !(dt > 0.0 && dt <= 30.0) {
            return;
        }
        let ncpu = snap.cpu.per_core.len().max(1) as f64;
        let today = day(0);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
        for g in &snap.groups {
            // stesso nome della versione Python, così la cronologia continua tra le due
            let app = match (&g.image, g.section) {
                (Some(img), Section::Ctr) => format!("{}  ·  {img}", g.label),
                _ => g.label.clone(),
            };
            let icon = g.icon.clone().unwrap_or_else(|| if g.section == Section::Ctr { "glyph:container".into() } else { String::new() });
            let cpu_s = g.cpu / 100.0 * ncpu * dt;
            let disk = g.disk.unwrap_or(0.0) * dt;
            let gpu_s = g.gpu.min(100.0) / 100.0 * dt;
            let a = self.acc.entry((today.clone(), app)).or_insert_with(|| Acc { kind: kind(g.section).into(), ..Default::default() });
            if a.icon.is_empty() {
                a.icon = icon;
            }
            a.cpu_s += cpu_s;
            a.disk_b += disk;
            a.gpu_s += gpu_s;
            a.mem_peak = a.mem_peak.max(g.mem as f64);
            if cpu_s > 0.005 || disk > 0.0 || gpu_s > 0.005 {
                a.last = now;
            }
        }
        if self.last_flush.elapsed() > Duration::from_secs(60) {
            self.flush();
        }
    }

    /// Scrive gli accumuli in un'unica transazione; in caso di errore li conserva per il tentativo successivo.
    fn flush(&mut self) {
        self.last_flush = Instant::now();
        let Some(conn) = self.conn.as_mut() else { return };
        if self.acc.is_empty() {
            return;
        }
        let acc = &self.acc;
        let res = (|| -> rusqlite::Result<()> {
            let tx = conn.transaction()?;
            {
                let mut st = tx.prepare_cached(UPSERT)?;
                for ((d, app), a) in acc {
                    st.execute(params![d, app, a.kind, a.icon, a.cpu_s, a.disk_b, a.gpu_s, a.mem_peak, a.last])?;
                }
            }
            tx.commit()
        })();
        if res.is_ok() {
            self.acc.clear();
        }
    }

    fn icon(&mut self, raw: &str, app: &str) -> Option<String> {
        if let Some(g) = raw.strip_prefix("svg:") {
            return Some(format!("glyph:{g}"));
        }
        if raw.starts_with("glyph:") {
            return Some(raw.to_string());
        }
        // la versione Python salvava il nome dell'icona: lo si risolve nel file del tema
        let ic = self.icons.get_or_insert_with(|| Icons::new(true));
        ic.resolve(raw).or_else(|| ic.resolve(&app.to_lowercase()))
    }

    /// Righe aggregate per app negli ultimi `days` giorni (1 = solo oggi).
    pub fn query(&mut self, days: u32) -> Vec<HistoryRow> {
        let since = day(days.max(1) as i64 - 1);
        let mut rows: Vec<(String, String, String, String, [f64; 5])> = vec![];
        if let Some(c) = &self.conn {
            let sql = "SELECT day, app, kind, icon, cpu_s, disk_b, gpu_s, mem_peak, last FROM usage WHERE day >= ?1 ORDER BY day";
            if let Ok(mut st) = c.prepare(sql) {
                let it = st.query_map([&since], |r| {
                    let f = |i: usize| r.get::<_, Option<f64>>(i).map(|v| v.unwrap_or(0.0));
                    Ok((
                        r.get::<_, Option<String>>(0)?.unwrap_or_default(),
                        r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                        r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                        [f(4)?, f(5)?, f(6)?, f(7)?, f(8)?],
                    ))
                });
                if let Ok(it) = it {
                    rows.extend(it.flatten());
                }
            }
        }
        for ((d, app), a) in &self.acc {
            if *d >= since {
                rows.push((d.clone(), app.clone(), a.kind.clone(), a.icon.clone(), [a.cpu_s, a.disk_b, a.gpu_s, a.mem_peak, a.last]));
            }
        }
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        let mut agg: HashMap<String, (String, String, [f64; 5])> = HashMap::new();
        for (_, app, kind, icon, v) in rows {
            let e = agg.entry(app).or_insert_with(|| (String::new(), String::new(), [0.0; 5]));
            if !kind.is_empty() {
                e.0 = kind;
            }
            if !icon.is_empty() {
                e.1 = icon;
            }
            for (a, b) in e.2.iter_mut().zip(v).take(3) {
                *a += b;
            }
            e.2[3] = e.2[3].max(v[3]);
            e.2[4] = e.2[4].max(v[4]);
        }
        let mut out: Vec<HistoryRow> = agg
            .into_iter()
            .filter(|(_, (_, _, v))| v[0] >= 0.5 || v[1] >= 1e6 || v[2] >= 0.5)
            .map(|(app, (kind, icon, v))| {
                let icon = self.icon(&icon, &app);
                HistoryRow { app, kind, icon, cpu_s: v[0], disk_b: v[1], gpu_s: v[2], mem_peak: v[3], last: v[4] }
            })
            .collect();
        out.sort_by(|a, b| b.cpu_s.total_cmp(&a.cpu_s).then_with(|| a.app.cmp(&b.app)));
        out
    }

    pub fn clear(&mut self) {
        self.acc.clear();
        if let Some(c) = &self.conn {
            let _ = c.execute("DELETE FROM usage", []);
        }
    }
}

impl Drop for History {
    fn drop(&mut self) {
        self.flush();
    }
}

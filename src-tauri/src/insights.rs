//! Diagnosis: when a resource is at its limit, explains who is consuming it and why.
use crate::fmt::{num, pct, rate, size};
use crate::model::{Culprit, Group, Insight, Section, Snapshot};

fn kind(g: &Group) -> &'static str {
    match g.section {
        Section::App => "app",
        Section::Ctr => "container",
        Section::Bg => "processo in background",
        Section::Sys => "servizio di sistema",
    }
}

fn culprit(g: &Group) -> Culprit {
    Culprit { key: g.key.clone(), label: g.label.clone(), kind: kind(g).into(), container: g.section == Section::Ctr }
}

fn top<'a, F: Fn(&Group) -> f64>(s: &'a Snapshot, f: F) -> Vec<&'a Group> {
    let mut v: Vec<&Group> = s.groups.iter().filter(|g| f(g) > 0.0).collect();
    v.sort_by(|a, b| f(b).total_cmp(&f(a)));
    v.truncate(4);
    v
}

pub fn analyze(s: &Snapshot) -> Vec<Insight> {
    let mut out = vec![];
    // ------------------------------------------------------------- disk
    let busy = s.disks.iter().max_by(|a, b| a.active.total_cmp(&b.active));
    if let Some(d) = busy.filter(|d| d.active >= 80.0) {
        let total = s.disks.iter().map(|d| d.read + d.write).sum::<f64>().max(1.0);
        let verb = if d.write >= d.read * 2.0 { "sta scrivendo" } else if d.read >= d.write * 2.0 { "sta leggendo" } else { "sta leggendo e scrivendo" };
        let level = if d.active >= 95.0 { "critical" } else { "warn" };
        let title = format!("Disco al {}%", pct(d.active));
        let swap = s.mem.swap_in + s.mem.swap_out;
        let t = top(s, |g| g.disk.unwrap_or(0.0));
        let others = |skip: &str| t.iter().filter(|g| g.key != skip).take(2).map(|g| format!("{} {}", g.label, rate(g.disk.unwrap_or(0.0)))).collect::<Vec<_>>();
        if swap > 4e6 && swap >= 0.3 * total {
            let m = top(s, |g| g.mem as f64);
            let sentence = match m.first() {
                Some(g) => format!(
                    "Il sistema sta spostando memoria su disco ({}) perché la RAM è al {}%. Chi ne occupa di più è {}, con {}.",
                    rate(swap), pct(s.mem.pct), g.label, size(g.mem)
                ),
                None => format!("Il sistema sta spostando memoria su disco ({}) perché la RAM è al {}%.", rate(swap), pct(s.mem.pct)),
            };
            out.push(Insight { res: "disk".into(), level: level.into(), title, sentence, culprit: m.first().map(|g| culprit(g)), others: vec![] });
        } else if let Some(g) = t.first().filter(|g| g.disk.unwrap_or(0.0) >= 0.25 * total) {
            let share = (g.disk.unwrap_or(0.0) / total * 100.0).min(100.0);
            let slow = if d.resp >= 20.0 { ", e le altre app ne risentono" } else { "" };
            let sentence = format!("{} {} {}: è il {}% di tutto l'I/O del disco{}.", g.label, verb, rate(g.disk.unwrap_or(0.0)), pct(share), slow);
            out.push(Insight { res: "disk".into(), level: level.into(), title, sentence, culprit: Some(culprit(g)), others: others(&g.key) });
        } else {
            let mut sentence = format!(
                "Il disco {} {} ma nessuna app spicca: è attività del kernel o del filesystem (scritture differite, journal, btrfs).",
                verb.trim_start_matches("sta "), rate(d.read + d.write)
            );
            if s.cpu.iowait >= 10.0 {
                sentence.push_str(&format!(" La CPU passa il {}% del tempo ad aspettare il disco.", pct(s.cpu.iowait)));
            }
            out.push(Insight { res: "disk".into(), level: level.into(), title, sentence, culprit: None, others: others("") });
        }
    }
    // ------------------------------------------------------------- memory
    if s.mem.pct >= 88.0 {
        let t = top(s, |g| g.mem as f64);
        if let Some(g) = t.first() {
            out.push(Insight {
                res: "mem".into(),
                level: if s.mem.pct >= 95.0 { "critical" } else { "warn" }.into(),
                title: format!("Memoria al {}%", pct(s.mem.pct)),
                sentence: format!(
                    "{} usa {}, il {}% della RAM; restano liberi {}.",
                    g.label, size(g.mem), pct(g.mem as f64 / s.mem.total.max(1) as f64 * 100.0), size(s.mem.available)
                ),
                culprit: Some(culprit(g)),
                others: t.iter().skip(1).take(2).map(|g| format!("{} {}", g.label, size(g.mem))).collect(),
            });
        }
    }
    // ------------------------------------------------------------- CPU and temperature
    let hot = s.cpu.temp.map(|t| t >= 95.0).unwrap_or(false);
    if s.cpu.total >= 85.0 || hot {
        let t = top(s, |g| g.cpu);
        if let Some(g) = t.first() {
            let others = t.iter().skip(1).take(2).map(|g| format!("{} {}%", g.label, num(g.cpu, 0))).collect();
            if hot {
                let hint = if g.section == Section::Ctr { "Metterlo in pausa la raffredda in pochi secondi." } else { "Ridurne il carico la raffredda." };
                out.push(Insight {
                    res: "temp".into(),
                    level: "critical".into(),
                    title: format!("CPU a {} °C", pct(s.cpu.temp.unwrap_or(0.0))),
                    sentence: format!("Il calore viene soprattutto da {}, che usa il {}% della CPU. {hint}", g.label, pct(g.cpu)),
                    culprit: Some(culprit(g)),
                    others,
                });
            } else {
                let share = g.cpu / s.cpu.total.max(1.0) * 100.0;
                let sentence = if share < 20.0 {
                    format!("Il carico è distribuito su molti processi; il più pesante è {}, con il {}%.", g.label, pct(g.cpu))
                } else {
                    format!("{} usa il {}% della CPU totale, il {}% del carico attuale.", g.label, pct(g.cpu), pct(share))
                };
                out.push(Insight {
                    res: "cpu".into(),
                    level: if s.cpu.total >= 95.0 { "critical" } else { "warn" }.into(),
                    title: format!("CPU al {}%", pct(s.cpu.total)),
                    sentence,
                    culprit: Some(culprit(g)),
                    others,
                });
            }
        }
    }
    // ------------------------------------------------------------- GPU
    let gmax = s.gpus.iter().map(|g| g.util).fold(0.0, f64::max);
    if gmax >= 85.0 {
        let t = top(s, |g| g.gpu);
        if let Some(g) = t.first() {
            out.push(Insight {
                res: "gpu".into(),
                level: "warn".into(),
                title: format!("GPU al {}%", pct(gmax)),
                sentence: format!("{} usa il {}% della GPU.", g.label, pct(g.gpu)),
                culprit: Some(culprit(g)),
                others: vec![],
            });
        }
    }
    let rank = |r: &str| ["disk", "mem", "temp", "cpu", "gpu"].iter().position(|x| *x == r).unwrap_or(9);
    out.sort_by_key(|i| (if i.level == "critical" { 0 } else { 1 }, rank(&i.res)));
    out
}

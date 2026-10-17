//! Diagnosis: when a resource is at its limit, explains who is consuming it and why.
use crate::fmt::{num, pct, rate, size};
use crate::i18n::{t, tf};
use crate::model::{Culprit, Group, Insight, Section, Snapshot};

fn kind(g: &Group) -> String {
    t(match g.section {
        Section::App => "insights.kind.app",
        Section::Ctr => "insights.kind.container",
        Section::Bg => "insights.kind.background",
        Section::Sys => "insights.kind.service",
    })
}

fn culprit(g: &Group) -> Culprit {
    Culprit { key: g.key.clone(), label: g.label.clone(), kind: kind(g), container: g.section == Section::Ctr }
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
        // what the disk is doing: selects the sentence variant ("write", "read" or "readWrite")
        let verb = if d.write >= d.read * 2.0 { "write" } else if d.read >= d.write * 2.0 { "read" } else { "readWrite" };
        let level = if d.active >= 95.0 { "critical" } else { "warn" };
        let title = tf("insights.disk.title", &[("pct", pct(d.active))]);
        let swap = s.mem.swap_in + s.mem.swap_out;
        let t = top(s, |g| g.disk.unwrap_or(0.0));
        let others = |skip: &str| t.iter().filter(|g| g.key != skip).take(2).map(|g| format!("{} {}", g.label, rate(g.disk.unwrap_or(0.0)))).collect::<Vec<_>>();
        if swap > 4e6 && swap >= 0.3 * total {
            let m = top(s, |g| g.mem as f64);
            let args = [("rate", rate(swap)), ("pct", pct(s.mem.pct))];
            let sentence = match m.first() {
                Some(g) => tf("insights.disk.swapTop", &[&args[..], &[("name", g.label.clone()), ("size", size(g.mem))]].concat()),
                None => tf("insights.disk.swap", &args),
            };
            out.push(Insight { res: "disk".into(), level: level.into(), title, sentence, culprit: m.first().map(|g| culprit(g)), others: vec![] });
        } else if let Some(g) = t.first().filter(|g| g.disk.unwrap_or(0.0) >= 0.25 * total) {
            let share = (g.disk.unwrap_or(0.0) / total * 100.0).min(100.0);
            let key = format!("insights.disk.{}.{verb}", if d.resp >= 20.0 { "appSlow" } else { "app" });
            let sentence = tf(&key, &[("name", g.label.clone()), ("rate", rate(g.disk.unwrap_or(0.0))), ("share", pct(share))]);
            out.push(Insight { res: "disk".into(), level: level.into(), title, sentence, culprit: Some(culprit(g)), others: others(&g.key) });
        } else {
            let mut sentence = tf(&format!("insights.disk.kernel.{verb}"), &[("rate", rate(d.read + d.write))]);
            if s.cpu.iowait >= 10.0 {
                sentence.push(' ');
                sentence.push_str(&tf("insights.disk.iowait", &[("pct", pct(s.cpu.iowait))]));
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
                title: tf("insights.mem.title", &[("pct", pct(s.mem.pct))]),
                sentence: tf(
                    "insights.mem.sentence",
                    &[
                        ("name", g.label.clone()),
                        ("size", size(g.mem)),
                        ("share", pct(g.mem as f64 / s.mem.total.max(1) as f64 * 100.0)),
                        ("free", size(s.mem.available)),
                    ],
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
                // a container can be paused, anything else can only be slowed down
                let key = if g.section == Section::Ctr { "insights.temp.container" } else { "insights.temp.other" };
                out.push(Insight {
                    res: "temp".into(),
                    level: "critical".into(),
                    title: tf("insights.temp.title", &[("temp", pct(s.cpu.temp.unwrap_or(0.0)))]),
                    sentence: tf(key, &[("name", g.label.clone()), ("pct", pct(g.cpu))]),
                    culprit: Some(culprit(g)),
                    others,
                });
            } else {
                let share = g.cpu / s.cpu.total.max(1.0) * 100.0;
                let sentence = if share < 20.0 {
                    tf("insights.cpu.spread", &[("name", g.label.clone()), ("pct", pct(g.cpu))])
                } else {
                    tf("insights.cpu.top", &[("name", g.label.clone()), ("pct", pct(g.cpu)), ("share", pct(share))])
                };
                out.push(Insight {
                    res: "cpu".into(),
                    level: if s.cpu.total >= 95.0 { "critical" } else { "warn" }.into(),
                    title: tf("insights.cpu.title", &[("pct", pct(s.cpu.total))]),
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
                title: tf("insights.gpu.title", &[("pct", pct(gmax))]),
                sentence: tf("insights.gpu.sentence", &[("name", g.label.clone()), ("pct", pct(g.gpu))]),
                culprit: Some(culprit(g)),
                others: vec![],
            });
        }
    }
    let rank = |r: &str| ["disk", "mem", "temp", "cpu", "gpu"].iter().position(|x| *x == r).unwrap_or(9);
    out.sort_by_key(|i| (if i.level == "critical" { 0 } else { 1 }, rank(&i.res)));
    out
}

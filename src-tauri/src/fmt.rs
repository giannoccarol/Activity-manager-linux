//! Numbers for the diagnosis sentences, in the interface language (Italian: 1.234,5; English: 1,234.5).
pub fn num(v: f64, dec: usize) -> String {
    let (thousands, decimal) = if crate::i18n::lang() == "it" { ('.', ',') } else { (',', '.') };
    let s = format!("{:.*}", dec, v.abs());
    let (int, frac) = s.split_once('.').map(|(a, b)| (a.to_string(), Some(b.to_string()))).unwrap_or((s.clone(), None));
    let mut grouped = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            grouped.push(thousands);
        }
        grouped.push(c);
    }
    let sign = if v < 0.0 { "-" } else { "" };
    match frac {
        Some(f) => format!("{sign}{grouped}{decimal}{f}"),
        None => format!("{sign}{grouped}"),
    }
}

pub fn pct(v: f64) -> String {
    num(v.round(), 0)
}

pub fn rate(bps: f64) -> String {
    if bps < 1048576.0 {
        format!("{} KB/s", num(bps / 1024.0, 0))
    } else if bps < 1073741824.0 {
        format!("{} MB/s", num(bps / 1048576.0, if bps < 100.0 * 1048576.0 { 1 } else { 0 }))
    } else {
        format!("{} GB/s", num(bps / 1073741824.0, 2))
    }
}

pub fn size(b: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < units.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} B", b)
    } else {
        format!("{} {}", num(v, if v >= 100.0 { 0 } else { 1 }), units[i])
    }
}

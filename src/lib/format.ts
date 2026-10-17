// Number and date formatting in the interface language (Italian: 1.234,5; English: 1,234.5).
import { i18n, t } from "./i18n.svelte";

const cache = new Map<string, Intl.NumberFormat>();
export function num(v: number, d = 1): string {
  const tag = i18n.tag;
  const k = `${tag}:${d}`;
  let f = cache.get(k);
  if (!f) cache.set(k, (f = new Intl.NumberFormat(tag, { minimumFractionDigits: d, maximumFractionDigits: d, useGrouping: "always" })));
  return f.format(v);
}

export const pct = (v: number) => (v < 0.05 ? "0" : num(v, v < 10 ? 1 : 0));
export const pct0 = (v: number) => num(Math.round(v), 0);

const UNITS = ["B", "KB", "MB", "GB", "TB"];
export function size(b: number, d = 1): [string, string] {
  let v = b;
  let i = 0;
  while (v >= 1024 && i < UNITS.length - 1) {
    v /= 1024;
    i++;
  }
  return [i === 0 ? String(Math.round(v)) : num(v, v >= 100 ? 0 : d), UNITS[i]];
}
export const sizeStr = (b: number, d = 1) => size(b, d).join(" ");

export function rate(bps: number): [string, string] {
  if (bps < 1024 * 1024) return [num(bps / 1024, 0), "KB/s"];
  if (bps < 1024 ** 3) return [num(bps / 1048576, bps < 100 * 1048576 ? 1 : 0), "MB/s"];
  return [num(bps / 1024 ** 3, 2), "GB/s"];
}
export const rateStr = (b: number) => rate(b).join(" ");

export function bits(bps: number): [string, string] {
  const b = bps * 8;
  if (b < 1e6) return [num(b / 1e3, 0), "Kbps"];
  if (b < 1e9) return [num(b / 1e6, 1), "Mbps"];
  return [num(b / 1e9, 2), "Gbps"];
}
export const bitsStr = (b: number) => bits(b).join(" ");

export const ghz = (mhz: number) => (mhz ? `${num(mhz / 1000, 2)} GHz` : "—");

export function uptime(sec: number): string {
  const d = Math.floor(sec / 86400);
  const h = Math.floor((sec % 86400) / 3600);
  const m = Math.floor((sec % 3600) / 60);
  if (d) return t("format.daysHours", { d, h });
  if (h) return t("format.hoursMinutes", { h, m });
  return t("format.minutes", { m });
}

/** Perceptual scale (square root): makes even small usage visible. */
export const meter = (v: number | null, full: number) => (!v || v <= 0 || !full ? 0 : Math.min(1, Math.sqrt(v / full)));

/** Readable duration: "45 s", "12 min", "2 h 14 min". */
export function dur(sec: number): string {
  if (sec < 1) return sec > 0 ? t("format.lessThanSecond") : t("format.seconds", { s: 0 });
  if (sec < 60) return t("format.seconds", { s: Math.round(sec) });
  const m = Math.floor(sec / 60);
  if (m < 60) return t("format.minutes", { m });
  const h = Math.floor(m / 60);
  return m % 60 ? t("format.hoursMinutes", { h, m: m % 60 }) : t("format.hours", { h });
}

/** How long ago, from a unix timestamp in seconds. */
export function ago(ts: number): string {
  if (!ts) return "—";
  const s = Date.now() / 1000 - ts;
  if (s < 60) return t("format.justNow");
  if (s < 3600) return t("format.minutesAgo", { m: Math.floor(s / 60) });
  const d = new Date(ts * 1000);
  const today = new Date();
  const hm = d.toLocaleTimeString(i18n.tag, { hour: "2-digit", minute: "2-digit" });
  if (d.toDateString() === today.toDateString()) return t("format.todayAt", { time: hm });
  const y = new Date(today);
  y.setDate(y.getDate() - 1);
  if (d.toDateString() === y.toDateString()) return t("format.yesterdayAt", { time: hm });
  return d.toLocaleDateString(i18n.tag, { day: "numeric", month: "long" });
}

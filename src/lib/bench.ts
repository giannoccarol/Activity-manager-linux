// UI performance measurement in the real app (gestione-attivita --bench).
import { tick } from "svelte";
import { app, type Page } from "./state.svelte";
import { flags } from "./motion.svelte";

async function cpu(): Promise<[number, number, number]> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<[number, number, number]>("bench_cpu");
}
/** app and KWin usage (in % of one core) and frames while staying idle for ms */
async function idleCost(ms: number) {
  const a = await cpu();
  const gaps = await gapsDuring(ms);
  const b = await cpu();
  const dt = b[2] - a[2];
  return `app ${f1(((b[0] - a[0]) / dt) * 100)}%, KWin ${f1(((b[1] - a[1]) / dt) * 100)}%, ${f1(1000 / (gaps.reduce((x, y) => x + y, 0) / gaps.length))} fps, ${gaps.filter((g) => g > 25).length} janks`;
}

export const tickCost: number[] = [];

const frame = () => new Promise<number>((r) => requestAnimationFrame(r));
async function settle() {
  await tick();
  await frame();
  await frame();
}
const f1 = (v: number) => v.toFixed(1);
function stats(a: number[]) {
  const s = [...a].sort((x, y) => x - y);
  const avg = s.reduce((x, y) => x + y, 0) / (s.length || 1);
  return `mean ${f1(avg)} ms, p95 ${f1(s[Math.floor(s.length * 0.95)] ?? 0)} ms, max ${f1(s[s.length - 1] ?? 0)} ms`;
}

/** Intervals between frames during an action (to count animation stutters). */
async function gapsDuring(ms: number): Promise<number[]> {
  const gaps: number[] = [];
  let prev = await frame();
  const end = prev + ms;
  while (prev < end) {
    const t = await frame();
    gaps.push(t - prev);
    prev = t;
  }
  return gaps;
}

/** Frames during an action: intervals between requestAnimationFrame calls. */
async function frames(ms: number, step?: (i: number) => void) {
  const gaps: number[] = [];
  let prev = await frame();
  const end = prev + ms;
  let i = 0;
  while (prev < end) {
    step?.(i++);
    const t = await frame();
    gaps.push(t - prev);
    prev = t;
  }
  const jank = gaps.filter((g) => g > 25).length;
  return `${gaps.length} frames, ${f1(1000 / (gaps.reduce((a, b) => a + b, 0) / gaps.length))} fps, ${jank} frames over 25 ms, worst ${f1(Math.max(...gaps))} ms`;
}

export async function runBench(): Promise<string> {
  const out: string[] = [];
  const pages: Page[] = ["processes", "performance", "history", "startup", "network", "users", "details", "services", "settings"];
  app.page = "processes";
  app.search = "";
  await settle();
  await new Promise((r) => setTimeout(r, 1500));

  out.push(`Idle (6 s, one update per second): ${await frames(6000)}`);
  // cost of each periodic animation: turned off one at a time, then all
  const html = document.documentElement;
  const variants: [string, () => void][] = [
    ["all on, window focused", () => {}],
    ["window not focused", () => {
      flags.focused = false;
      html.classList.add("calm");
    }],
    ["all off", () => {
      html.classList.add("no-meter-anim");
      flags.glide = flags.flip = flags.tween = false;
    }],
  ];
  for (const [name, set] of variants) {
    html.classList.remove("no-meter-anim", "calm");
    flags.glide = flags.flip = flags.tween = flags.focused = true;
    set();
    await new Promise((r) => setTimeout(r, 600));
    out.push(`Animations ${name}: ${await idleCost(6000)}`);
  }
  html.classList.remove("no-meter-anim", "calm");
  flags.glide = flags.flip = flags.tween = flags.focused = true;
  out.push(`Cost of one update (data + rendering): ${stats(tickCost.slice(-20))}`);

  const animGaps: number[] = [];
  for (const round of ["first", "second"]) {
    const t: string[] = [];
    for (const p of pages) {
      const t0 = performance.now();
      app.page = p;
      await settle();
      t.push(`${p} ${f1(performance.now() - t0)}`);
      // the entrance animation lasts ~260 ms: count slow frames while it runs
      animGaps.push(...(await gapsDuring(320)));
    }
    out.push(`Page switch (${round} round, ms): ${t.join(", ")}`);
  }
  out.push(`During page animations: ${animGaps.length} frames, ${animGaps.filter((g) => g > 25).length} over 25 ms, worst ${f1(Math.max(...animGaps))} ms`);

  app.page = "processes";
  await settle();
  const typed: string[] = [];
  for (const q of ["c", "ch", "chr", "chro", "chrom", "chrome", "", "k", "kw", "kwi", "kwin", ""]) {
    const t0 = performance.now();
    app.search = q;
    await settle();
    typed.push(f1(performance.now() - t0));
  }
  out.push(`Search, per keystroke (ms): ${typed.join(", ")}`);
  // animated reordering: click the Nome and CPU headers, then watch the frames
  const heads = [...document.querySelectorAll<HTMLButtonElement>(".head .h")];
  const sortGaps: number[] = [];
  for (const label of ["Nome", "CPU", "Memoria", "CPU"]) {
    heads.find((h) => h.textContent?.trim().startsWith(label))?.click();
    sortGaps.push(...(await gapsDuring(450)));
  }
  out.push(`Animated re-sorting (4 clicks): ${sortGaps.length} frames, ${sortGaps.filter((g) => g > 25).length} over 25 ms, worst ${f1(Math.max(...sortGaps))} ms`);

  const rows = document.querySelector("[data-scroll]") as HTMLElement | null;
  if (rows) {
    rows.scrollTop = 0;
    const max = rows.scrollHeight - rows.clientHeight;
    out.push(`Scrolling Processes (${rows.scrollHeight} px): ${await frames(3000, (i) => (rows.scrollTop = ((i * 24) % (2 * max)) < max ? (i * 24) % max : max - ((i * 24) % max)))}`);
    rows.scrollTop = 0;
  }

  app.page = "details";
  await settle();
  const drows = document.querySelector("[data-scroll]") as HTMLElement | null;
  if (drows) {
    const max = drows.scrollHeight - drows.clientHeight;
    out.push(`Scrolling Details (${drows.scrollHeight} px): ${await frames(3000, (i) => (drows.scrollTop = (i * 24) % Math.max(1, max)))}`);
  }
  out.push(`DOM elements on Details: ${document.querySelectorAll("*").length}`);
  app.page = "processes";
  await settle();
  out.push(`DOM elements on Processes: ${document.querySelectorAll("*").length}`);
  return out.join("\n");
}

// Misura delle prestazioni dell'interfaccia nell'app vera (gestione-attivita --bench).
import { tick } from "svelte";
import { app, type Page } from "./state.svelte";
import { flags } from "./motion.svelte";

async function cpu(): Promise<[number, number, number]> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<[number, number, number]>("bench_cpu");
}
/** consumo di app e KWin (in % di un core) e fotogrammi mentre si resta fermi per ms */
async function idleCost(ms: number) {
  const a = await cpu();
  const gaps = await gapsDuring(ms);
  const b = await cpu();
  const dt = b[2] - a[2];
  return `app ${f1(((b[0] - a[0]) / dt) * 100)}%, KWin ${f1(((b[1] - a[1]) / dt) * 100)}%, ${f1(1000 / (gaps.reduce((x, y) => x + y, 0) / gaps.length))} fps, ${gaps.filter((g) => g > 25).length} scatti`;
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
  return `media ${f1(avg)} ms, p95 ${f1(s[Math.floor(s.length * 0.95)] ?? 0)} ms, max ${f1(s[s.length - 1] ?? 0)} ms`;
}

/** Intervalli tra fotogrammi durante un'azione (per contare gli scatti delle animazioni). */
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

/** Fotogrammi durante un'azione: intervalli tra requestAnimationFrame. */
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
  return `${gaps.length} fotogrammi, ${f1(1000 / (gaps.reduce((a, b) => a + b, 0) / gaps.length))} fps, ${jank} scatti oltre 25 ms, peggiore ${f1(Math.max(...gaps))} ms`;
}

export async function runBench(): Promise<string> {
  const out: string[] = [];
  const pages: Page[] = ["processes", "performance", "history", "startup", "network", "users", "details", "services", "settings"];
  app.page = "processes";
  app.search = "";
  await settle();
  await new Promise((r) => setTimeout(r, 1500));

  out.push(`Inattiva (6 s, aggiornamento ogni secondo): ${await frames(6000)}`);
  // peso di ogni animazione periodica: si spengono una alla volta, poi tutte
  const html = document.documentElement;
  const variants: [string, () => void][] = [
    ["tutte accese, finestra in primo piano", () => {}],
    ["finestra non in primo piano", () => {
      flags.focused = false;
      html.classList.add("calm");
    }],
    ["tutte spente", () => {
      html.classList.add("no-meter-anim");
      flags.glide = flags.flip = flags.tween = false;
    }],
  ];
  for (const [name, set] of variants) {
    html.classList.remove("no-meter-anim", "calm");
    flags.glide = flags.flip = flags.tween = flags.focused = true;
    set();
    await new Promise((r) => setTimeout(r, 600));
    out.push(`Animazioni ${name}: ${await idleCost(6000)}`);
  }
  html.classList.remove("no-meter-anim", "calm");
  flags.glide = flags.flip = flags.tween = flags.focused = true;
  out.push(`Costo di un aggiornamento (dati + disegno): ${stats(tickCost.slice(-20))}`);

  const animGaps: number[] = [];
  for (const round of ["primo", "secondo"]) {
    const t: string[] = [];
    for (const p of pages) {
      const t0 = performance.now();
      app.page = p;
      await settle();
      t.push(`${p} ${f1(performance.now() - t0)}`);
      // l'animazione di entrata dura ~260 ms: si contano i fotogrammi lenti mentre scorre
      animGaps.push(...(await gapsDuring(320)));
    }
    out.push(`Cambio pagina (${round} giro, ms): ${t.join(", ")}`);
  }
  out.push(`Durante le animazioni di pagina: ${animGaps.length} fotogrammi, ${animGaps.filter((g) => g > 25).length} oltre 25 ms, peggiore ${f1(Math.max(...animGaps))} ms`);

  app.page = "processes";
  await settle();
  const typed: string[] = [];
  for (const q of ["c", "ch", "chr", "chro", "chrom", "chrome", "", "k", "kw", "kwi", "kwin", ""]) {
    const t0 = performance.now();
    app.search = q;
    await settle();
    typed.push(f1(performance.now() - t0));
  }
  out.push(`Ricerca, per tasto (ms): ${typed.join(", ")}`);
  // riordino animato: clic sulle intestazioni Nome e CPU, poi si osservano i fotogrammi
  const heads = [...document.querySelectorAll<HTMLButtonElement>(".head .h")];
  const sortGaps: number[] = [];
  for (const label of ["Nome", "CPU", "Memoria", "CPU"]) {
    heads.find((h) => h.textContent?.trim().startsWith(label))?.click();
    sortGaps.push(...(await gapsDuring(450)));
  }
  out.push(`Riordino animato (4 clic): ${sortGaps.length} fotogrammi, ${sortGaps.filter((g) => g > 25).length} oltre 25 ms, peggiore ${f1(Math.max(...sortGaps))} ms`);

  const rows = document.querySelector("[data-scroll]") as HTMLElement | null;
  if (rows) {
    rows.scrollTop = 0;
    const max = rows.scrollHeight - rows.clientHeight;
    out.push(`Scorrimento processi (${rows.scrollHeight} px): ${await frames(3000, (i) => (rows.scrollTop = ((i * 24) % (2 * max)) < max ? (i * 24) % max : max - ((i * 24) % max)))}`);
    rows.scrollTop = 0;
  }

  app.page = "details";
  await settle();
  const drows = document.querySelector("[data-scroll]") as HTMLElement | null;
  if (drows) {
    const max = drows.scrollHeight - drows.clientHeight;
    out.push(`Scorrimento Dettagli (${drows.scrollHeight} px): ${await frames(3000, (i) => (drows.scrollTop = (i * 24) % Math.max(1, max)))}`);
  }
  out.push(`Elementi nel DOM in Dettagli: ${document.querySelectorAll("*").length}`);
  app.page = "processes";
  await settle();
  out.push(`Elementi nel DOM in Processi: ${document.querySelectorAll("*").length}`);
  return out.join("\n");
}

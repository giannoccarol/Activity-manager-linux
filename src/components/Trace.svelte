<script lang="ts">
  // Oscilloscope-style trace with an instrument graticule. On each sample it glides one step in
  // 240 ms and then stops: smooth motion, but for three quarters of the time nothing is redrawn.
  import { ambient, flags } from "../lib/motion.svelte";
  import type { Sample } from "../lib/state.svelte";
  import { t } from "../lib/i18n.svelte";

  type Props = {
    samples: Sample[];
    second?: Sample[];
    now: number;
    max?: number | "auto";
    floor?: number;
    color: string;
    window?: number;
    graticule?: "none" | "scale" | "full";
    fill?: boolean;
    format?: (v: number) => string;
    names?: [string, string];
    draw?: boolean;
    onscale?: (v: number) => void;
  };
  let {
    samples,
    second,
    now,
    max = 100,
    floor = 1,
    color,
    window = 60,
    graticule = "scale",
    fill = true,
    format = (v: number) => `${Math.round(v)}%`,
    names,
    draw = false,
    onscale,
  }: Props = $props();

  const uid = $props.id();
  let w = $state(0);
  let h = $state(0);
  let hoverX = $state<number | null>(null);
  let layer: HTMLDivElement | undefined = $state();
  let prevNow = 0;

  function niceCeil(v: number, fl: number): number {
    v = Math.max(v, fl);
    const mag = Math.pow(10, Math.floor(Math.log10(v)));
    for (const m of [1, 2, 5, 10]) if (m * mag >= v) return m * mag;
    return 10 * mag;
  }

  const scale = $derived.by(() => {
    if (max !== "auto") return max;
    let m = 0;
    for (const arr of [samples, second ?? []]) for (const [t, v] of arr) if (now - t <= window + 1 && v > m) m = v;
    return niceCeil(m * 1.12, floor);
  });
  $effect(() => onscale?.(scale));

  function points(arr: Sample[]): [number, number][] {
    const out: [number, number][] = [];
    const pad = 1.5;
    for (const [t, v] of arr) {
      const age = now - t;
      if (age > window + 3) continue;
      const x = w - (age / window) * w;
      const y = h - pad - (Math.max(0, Math.min(v, scale)) / scale) * (h - pad * 2);
      out.push([x, y]);
    }
    return out;
  }

  function line(pts: [number, number][]): string {
    if (pts.length < 2) return "";
    let d = `M${pts[0][0].toFixed(1)},${pts[0][1].toFixed(1)}`;
    for (let i = 1; i < pts.length; i++) {
      const [x0, y0] = pts[i - 1];
      const [x1, y1] = pts[i];
      const xm = (x0 + x1) / 2;
      d += `C${xm.toFixed(1)},${y0.toFixed(1)} ${xm.toFixed(1)},${y1.toFixed(1)} ${x1.toFixed(1)},${y1.toFixed(1)}`;
    }
    return d;
  }

  const p1 = $derived(points(samples));
  const d1 = $derived(line(p1));
  const area = $derived(p1.length > 1 ? `${d1}L${p1[p1.length - 1][0].toFixed(1)},${h}L${p1[0][0].toFixed(1)},${h}Z` : "");
  const d2 = $derived(second ? line(points(second)) : "");

  $effect(() => {
    const n = now;
    if (!layer || !w) return;
    const dx = prevNow ? ((n - prevNow) / window) * w : 0;
    prevNow = n;
    if (dx <= 0 || dx > w / 4 || !ambient() || !flags.glide) return;
    layer.style.transition = "none";
    layer.style.transform = `translateX(${dx}px)`;
    void layer.offsetWidth;
    layer.style.transition = "transform 240ms cubic-bezier(0.22, 1, 0.36, 1)";
    layer.style.transform = "translateX(0)";
  });

  const hover = $derived.by(() => {
    if (hoverX === null || !samples.length) return null;
    const age = ((w - hoverX) / w) * window;
    const target = now - age;
    let best: Sample | null = null;
    for (const s of samples) if (!best || Math.abs(s[0] - target) < Math.abs(best[0] - target)) best = s;
    if (!best || Math.abs(best[0] - target) > 3) return null;
    let b2: Sample | null = null;
    if (second) for (const s of second) if (!b2 || Math.abs(s[0] - best[0]) < Math.abs(b2[0] - best[0])) b2 = s;
    const x = w - ((now - best[0]) / window) * w;
    const y = h - 1.5 - (Math.min(best[1], scale) / scale) * (h - 3);
    const secs = Math.round(now - best[0]);
    return { x, y, v: best[1], v2: b2?.[1], secs };
  });

  const ticks = $derived(Array.from({ length: 13 }, (_, i) => (i / 12) * w));
</script>

<div
  class="trace"
  class:full={graticule === "full"}
  bind:clientWidth={w}
  bind:clientHeight={h}
  role="img"
  aria-label={t("chart.label", { seconds: window })}
  onpointermove={(e) => graticule === "full" && (hoverX = e.offsetX)}
  onpointerleave={() => (hoverX = null)}
  style="--tc:{color}">
  {#if w && h}
    <svg class="grid" width={w} height={h} aria-hidden="true">
      {#if graticule === "full"}
        {#each [0.25, 0.5, 0.75] as f}
          <line x1="0" x2={w} y1={h * f} y2={h * f} class:axis={f === 0.5} />
        {/each}
        {#each ticks as x, i}
          {#if i > 0 && i < 12}<line x1={x} x2={x} y1="0" y2={h} class:axis={i === 6} />{/if}
        {/each}
      {:else if graticule === "scale"}
        <line x1="0" x2={w} y1={h * 0.5} y2={h * 0.5} class="dash" />
        {#each ticks as x, i}
          <line x1={x} x2={x} y1={h} y2={h - (i % 6 === 0 ? 5 : 2.5)} class="tick" />
        {/each}
      {/if}
    </svg>
    <div class="layer" bind:this={layer}>
      <svg width={w} height={h} aria-hidden="true" class:drawn={draw}>
        <defs>
          <linearGradient id="g{uid}" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" style="stop-color: var(--tc); stop-opacity: 0.22" />
            <stop offset="1" style="stop-color: var(--tc); stop-opacity: 0" />
          </linearGradient>
        </defs>
        {#if fill && area}<path d={area} fill="url(#g{uid})" class="area" />{/if}
        {#if d2}<path d={d2} class="line second" />{/if}
        {#if d1}<path d={d1} class="line" pathLength={draw ? 1 : undefined} />{/if}
      </svg>
    </div>
    {#if hover}
      <div class="cross" style="left:{hover.x}px"></div>
      <div class="dot" style="left:{hover.x}px; top:{hover.y}px"></div>
      <div class="tip" class:left={hover.x > w * 0.6} style="left:{hover.x}px">
        <strong>{names ? `${names[0]} ` : ""}{format(hover.v)}</strong>
        {#if hover.v2 !== undefined}<span>{names ? `${names[1]} ` : ""}{format(hover.v2)}</span>{/if}
        <span class="when">{hover.secs < 1 ? t("chart.now") : t("chart.secondsAgo", { n: hover.secs })}</span>
      </div>
    {/if}
  {/if}
</div>

<style>
  .trace {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    contain: strict;
  }
  .grid {
    position: absolute;
    inset: 0;
  }
  .grid line {
    stroke: var(--line-soft);
    stroke-width: 1;
    shape-rendering: crispEdges;
  }
  .grid line.axis {
    stroke: var(--line);
  }
  .grid line.dash {
    stroke: var(--line-soft);
    stroke-dasharray: 2 4;
  }
  .grid line.tick {
    stroke: var(--line);
  }
  .layer {
    position: absolute;
    inset: 0;
    will-change: transform;
  }
  .line {
    fill: none;
    stroke: var(--tc);
    stroke-width: 1.5;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
  .full .line {
    stroke-width: 1.75;
  }
  .line.second {
    stroke-dasharray: 3 3;
    opacity: 0.8;
  }
  .drawn .line {
    stroke-dasharray: 1;
    animation: draw 900ms var(--ease) both;
  }
  .drawn .line.second {
    stroke-dasharray: 3 3;
    animation: none;
  }
  .drawn .area {
    animation: fade 900ms var(--ease) both;
  }
  @keyframes draw {
    from {
      stroke-dashoffset: 1;
    }
    to {
      stroke-dashoffset: 0;
    }
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  .cross {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 1px;
    background: var(--faint);
    pointer-events: none;
  }
  .dot {
    position: absolute;
    width: 7px;
    height: 7px;
    margin: -3.5px 0 0 -3.5px;
    border-radius: 50%;
    background: var(--tc);
    box-shadow: 0 0 0 2px var(--panel);
    pointer-events: none;
  }
  .tip {
    position: absolute;
    top: 8px;
    transform: translateX(10px);
    display: flex;
    gap: 10px;
    align-items: baseline;
    white-space: nowrap;
    padding: 5px 9px;
    border-radius: var(--r-sm);
    background: var(--raised);
    border: 1px solid var(--line);
    font-size: var(--fs-sm);
    color: var(--text-2);
    pointer-events: none;
  }
  .tip.left {
    transform: translateX(calc(-100% - 10px));
  }
  .tip strong {
    color: var(--text);
    font-weight: 600;
  }
  .tip .when {
    color: var(--muted);
  }
</style>

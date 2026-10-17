<script lang="ts">
  import { ambient, flags } from "../lib/motion.svelte";
  import { Tween } from "svelte/motion";
  import { cubicOut } from "svelte/easing";
  import { app } from "../lib/state.svelte";
  import { bits, num, pct0, rate, size } from "../lib/format";
  import type { Resource } from "../lib/types";
  import Trace from "./Trace.svelte";
  import { t } from "../lib/i18n.svelte";

  let { onopen }: { onopen?: (r: Resource) => void } = $props();

  const s = $derived(app.snap!);
  const alertRes = $derived(new Map(s.insights.map((i) => [i.res === "temp" ? "cpu" : i.res, i.level])));

  type Seg = {
    key: Resource;
    name: string;
    value: number;
    text: string;
    unit: string;
    detail: string[];
    hot?: boolean;
    max: number | "auto";
  };

  const segs = $derived.by((): Seg[] => {
    const nets = s.nets.filter((n) => n.connected && n.kind !== "VPN");
    const rx = nets.reduce((a, n) => a + n.recv, 0);
    const tx = nets.reduce((a, n) => a + n.send, 0);
    const [nv, nu] = bits(rx + tx);
    const disk = s.disks.reduce((a, d) => (d.active > a.active ? d : a), s.disks[0]);
    const g = s.gpus.reduce((a, x) => (x.util > a.util ? x : a), s.gpus[0]);
    const [mu, mt] = [size(s.mem.used), size(s.mem.total)];
    const verb = disk && disk.write > disk.read * 2 ? "strip.writing" : "strip.reading";
    return [
      {
        key: "cpu",
        name: "CPU",
        value: s.cpu.total,
        text: pct0(s.cpu.total),
        unit: "%",
        detail: [`${num(s.cpu.freq / 1000, 2)} GHz`, s.cpu.temp !== null ? `${pct0(s.cpu.temp)} °C` : ""],
        hot: (s.cpu.temp ?? 0) >= 90,
        max: 100,
      },
      { key: "mem", name: t("common.memory"), value: s.mem.pct, text: pct0(s.mem.pct), unit: "%", detail: [t("common.ofTotal", { used: mu[0], total: mt.join(" ") })], max: 100 },
      {
        key: "disk",
        name: t("common.disk"),
        value: disk?.active ?? 0,
        text: pct0(disk?.active ?? 0),
        unit: "%",
        detail: disk ? [t(verb, { rate: rate(Math.max(disk.read, disk.write)).join(" ") })] : [],
        max: 100,
      },
      { key: "net", name: t("common.network"), value: rx + tx, text: nv, unit: nu, detail: [`↓ ${bits(rx).join(" ")}`, `↑ ${bits(tx).join(" ")}`], max: "auto" },
      {
        key: "gpu",
        name: "GPU",
        value: g?.util ?? 0,
        text: pct0(g?.util ?? 0),
        unit: "%",
        detail: g ? [g.name.replace(" Graphics", ""), size(g.memUsed).join(" ")] : [],
        max: 100,
      },
    ];
  });

  // a single animated moment: on open the numbers count up; then they update without animation
  const tweens: Record<string, Tween<number>> = {};
  let opened = false;
  for (const k of ["cpu", "mem", "disk", "net", "gpu"]) tweens[k] = new Tween(0, { duration: 800, easing: cubicOut });
  $effect(() => {
    for (const sg of segs) {
      const t = tweens[sg.key];
      if (!opened) t.target = sg.value;
      else t.set(sg.value, { duration: flags.tween && ambient() ? 240 : 0 });
    }
    if (!opened) setTimeout(() => (opened = true), 820);
  });

  function shown(sg: Seg): string {
    const v = tweens[sg.key].current;
    if (sg.key === "net") return bits(v)[0];
    return pct0(v);
  }
</script>

<section class="strip" aria-label={t("strip.label")}>
  {#each segs as sg (sg.key)}
    {@const level = alertRes.get(sg.key)}
    <button class="seg {level ?? ''}" style="--c: var(--{sg.key})" onclick={() => onopen?.(sg.key)}>
      <span class="name">{sg.name}</span>
      <span class="readout">
        <span class="value">{shown(sg)}</span><span class="unit">{sg.unit}</span>
      </span>
      <span class="detail">
        {#each sg.detail.filter(Boolean) as d, i}
          <span class:hot={sg.hot && i === 1}>{d}</span>
        {/each}
      </span>
      <span class="trace">
        <Trace samples={app.hist[sg.key] ?? []} now={s.t} max={sg.max} floor={125000} color="var(--c)" draw={app.count < 3} />
      </span>
    </button>
  {/each}
</section>

<style>
  .strip {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    background: var(--panel-bg);
    box-shadow: var(--panel-shadow);
    border: 1px solid var(--panel-edge);
    border-radius: var(--r-md);
    overflow: hidden;
  }
  .seg {
    position: relative;
    display: grid;
    grid-template-rows: auto auto auto 46px;
    text-align: left;
    padding: 12px 16px 0;
    min-width: 0;
    border-left: 1px solid var(--line-soft);
    transition: background-color 120ms;
  }
  .seg:first-child {
    border-left: 0;
  }
  .seg:hover {
    background: var(--hover);
  }
  .seg.warn::before,
  .seg.critical::before {
    content: "";
    position: absolute;
    inset: 0 0 auto 0;
    height: 2px;
    background: var(--warn);
    transform-origin: left;
    animation: rule-in var(--d-large) var(--ease-out) both;
  }
  @keyframes rule-in {
    from {
      transform: scaleX(0);
    }
  }
  .seg.critical::before {
    background: var(--crit);
  }
  .name {
    color: var(--text-2);
    font-size: var(--fs-base);
  }
  .readout {
    display: flex;
    align-items: baseline;
    gap: 2px;
    margin-top: 2px;
  }
  .value {
    font-size: 30px;
    font-weight: 550;
    font-stretch: 82%;
    letter-spacing: -0.01em;
    line-height: 1.1;
  }
  .seg.warn .value {
    color: var(--warn);
  }
  .seg.critical .value {
    color: var(--crit);
  }
  .unit {
    font-size: var(--fs-md);
    color: var(--muted);
    font-stretch: 90%;
  }
  .detail {
    display: flex;
    gap: 10px;
    margin-top: 2px;
    color: var(--muted);
    font-size: var(--fs-sm);
    white-space: nowrap;
    overflow: hidden;
  }
  .detail .hot {
    color: var(--warn);
  }
  .trace {
    display: block;
    margin: 8px -16px 0;
  }
</style>

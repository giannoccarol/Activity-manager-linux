<script lang="ts" module>
  import type { Sample } from "../lib/state.svelte";
  // history of every reading, kept while the app is open so the chart is full when you come back
  const history = new Map<string, Sample[]>();
  const KEEP = 150;
</script>

<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "../lib/api";
  import { settings } from "../lib/settings.svelte";
  import { num, sizeStr } from "../lib/format";
  import { t } from "../lib/i18n.svelte";
  import { app } from "../lib/state.svelte";
  import { mockSensors } from "../lib/mockpages";
  import type { SensorDevice, SensorReading } from "../lib/types";
  import Icon from "../components/Icon.svelte";
  import Trace from "../components/Trace.svelte";

  // Every sensor of the PC, device by device, like HWiNFO: current value, minimum, maximum and
  // average since the app started (or since the last reset).
  let devices = $state.raw<SensorDevice[]>([]);
  let loaded = $state(false);
  let selected = $state<string | null>(null);
  let closed = $state<Record<string, boolean>>({});
  let now = $state(0);
  let tick = $state(0);

  async function load() {
    try {
      const list = await invoke<SensorDevice[]>("sensors", {}, mockSensors);
      const ts = performance.now() / 1000;
      for (const d of list)
        for (const r of d.readings) {
          const h = history.get(r.id) ?? [];
          h.push([ts, r.value]);
          if (h.length > KEEP) h.splice(0, h.length - KEEP);
          history.set(r.id, h);
        }
      now = ts;
      devices = list;
      tick++;
      if (!selected) selected = list.flatMap((d) => d.readings).find((r) => r.kind === "temp")?.id ?? null;
    } catch {}
    loaded = true;
  }

  onMount(() => {
    load();
    const ms = Math.max(500, settings.v.intervalMs);
    const id = setInterval(load, ms);
    return () => clearInterval(id);
  });

  async function reset() {
    await invoke("sensors_reset", {}, () => null).catch(() => {});
    load();
  }

  const UNIT: Record<string, string> = { temp: "°C", fan: "RPM", volt: "V", current: "A", power: "W", clock: "MHz", percent: "%", energy: "Wh", count: "", memory: "" };
  function fmt(r: SensorReading, v: number): [string, string] {
    switch (r.kind) {
      case "temp":
        return [num(v, 0), UNIT.temp];
      case "volt":
      case "current":
        return [num(v, 2), UNIT[r.kind]];
      case "power":
      case "energy":
        return [num(v, 1), UNIT[r.kind]];
      case "percent":
        return [num(v, v < 10 ? 1 : 0), UNIT.percent];
      case "memory": {
        const [a, b] = sizeStr(v).split(" ");
        return [a, b ?? ""];
      }
      default:
        return [num(v, 0), UNIT[r.kind] ?? ""];
    }
  }
  const level = (r: SensorReading, v: number) => (r.crit != null && v >= r.crit ? "crit" : r.warn != null && v >= r.warn ? "warn" : "");
  const fill = (r: SensorReading) => (r.kind === "temp" ? Math.min(1, r.value / ((r.crit ?? 90) + 3)) : r.kind === "percent" ? Math.min(1, r.value / 100) : null);

  const all = $derived(devices.flatMap((d) => d.readings.map((r) => ({ r, d }))));
  const cur = $derived(all.find((x) => x.r.id === selected) ?? all[0] ?? null);
  const curHist = $derived.by(() => {
    tick;
    return cur ? [...(history.get(cur.r.id) ?? [])] : [];
  });
  const needle = $derived(app.search.trim().toLowerCase());
  const visible = $derived(
    needle
      ? devices
          .map((d) => ({ ...d, readings: d.name.toLowerCase().includes(needle) ? d.readings : d.readings.filter((r) => r.label.toLowerCase().includes(needle) || (r.sub ?? "").toLowerCase().includes(needle)) }))
          .filter((d) => d.readings.length)
      : devices,
  );
  const hottest = $derived(all.filter((x) => x.r.kind === "temp").sort((a, b) => b.r.value - a.r.value)[0] ?? null);
  const ICON: Record<string, string> = { cpu: "chip", gpu: "gpu", memory: "memory", disk: "disk", network: "network", board: "board", battery: "battery" };
</script>

{#snippet value(r: SensorReading, v: number)}
  {@const [n, u] = fmt(r, v)}
  <span class="v {level(r, v)}">{n}{#if u}<small>{u}</small>{/if}</span>
{/snippet}

<div class="pg">
  <div class="pg-bar">
    <span class="note">
      {t("sensors.note")}
      {#if hottest}<span class="hot">{t("sensors.hottest", { name: hottest.r.label, device: hottest.d.name, value: fmt(hottest.r, hottest.r.value).join(" ") })}</span>{/if}
    </span>
    <div class="pg-actions">
      <button class="btn" onclick={reset}><Icon name="restart" size={13} />{t("sensors.reset")}</button>
    </div>
  </div>

  {#if cur}
    <section class="chart pg-panel">
      <div class="head">
        <div class="who">
          <span class="dev">{cur.d.name}</span>
          <span class="name">{cur.r.label}{#if cur.r.sub}<span class="sub">{cur.r.sub}</span>{/if}</span>
        </div>
        <div class="big">{@render value(cur.r, cur.r.value)}</div>
        <dl>
          <div><dt>{t("sensors.cols.min")}</dt><dd>{@render value(cur.r, cur.r.min)}</dd></div>
          <div><dt>{t("sensors.cols.max")}</dt><dd>{@render value(cur.r, cur.r.max)}</dd></div>
          <div><dt>{t("sensors.cols.avg")}</dt><dd>{@render value(cur.r, cur.r.avg)}</dd></div>
        </dl>
      </div>
      <div class="graph">
        <Trace samples={curHist} {now} window={120} max="auto" floor={cur.r.kind === "temp" ? 20 : 1} color="var(--{cur.r.kind === 'temp' ? 'gpu' : cur.r.kind === 'clock' ? 'cpu' : cur.r.kind === 'fan' ? 'mem' : 'net'})" graticule="full" format={(v) => fmt(cur.r, v).join(" ")} />
      </div>
    </section>
  {/if}

  <div class="pg-panel table" role="treegrid" aria-label={t("nav.sensors")}>
    <div class="row headrow" role="row">
      <span role="columnheader">{t("sensors.cols.sensor")}</span>
      <span role="columnheader" class="r">{t("sensors.cols.current")}</span>
      <span role="columnheader" class="r">{t("sensors.cols.min")}</span>
      <span role="columnheader" class="r">{t("sensors.cols.max")}</span>
      <span role="columnheader" class="r">{t("sensors.cols.avg")}</span>
    </div>
    <div class="rows" data-scroll>
      {#each visible as d (d.id)}
        {@const open = !closed[d.id] || !!needle}
        <button class="device" aria-expanded={open} onclick={() => (closed[d.id] = !closed[d.id])}>
          <span class="chev" class:open><Icon name="chev-right" size={14} stroke={2} /></span>
          <span class="dicon"><Icon name={ICON[d.kind] ?? "board"} size={16} /></span>
          <span class="dname">{d.name}</span>
          {#if d.note}<span class="dnote">{d.note}</span>{/if}
          <span class="dcount">{d.readings.length}</span>
        </button>
        {#if open}
          {#each d.readings as r (r.id)}
            {@const f = fill(r)}
            <div class="row reading" class:selected={selected === r.id} role="row" tabindex="-1" onclick={() => (selected = r.id)} onkeydown={(e) => e.key === "Enter" && (selected = r.id)}>
              <span class="label" role="gridcell">
                <span class="t">{r.label}</span>
                {#if r.sub}<span class="sub">{r.sub}</span>{/if}
              </span>
              <span class="r cur" role="gridcell">
                {@render value(r, r.value)}
                {#if f !== null}<span class="meter {level(r, r.value)}"><i style="transform: scaleX({f})"></i></span>{/if}
              </span>
              <span class="r" role="gridcell">{@render value(r, r.min)}</span>
              <span class="r" role="gridcell">{@render value(r, r.max)}</span>
              <span class="r" role="gridcell">{@render value(r, r.avg)}</span>
            </div>
          {/each}
        {/if}
      {:else}
        <p class="empty">{loaded ? t("sensors.empty") : t("sensors.loading")}</p>
      {/each}
    </div>
  </div>
</div>

<style>
  .note {
    display: flex;
    gap: 14px;
    align-items: baseline;
    min-width: 0;
  }
  .hot {
    color: var(--text-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .chart {
    flex: 0 0 auto;
    display: grid;
    grid-template-columns: minmax(220px, 300px) minmax(0, 1fr);
    gap: 18px;
    height: 176px;
    padding: 16px 18px;
  }
  .head {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .who {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .dev {
    color: var(--muted);
    font-size: var(--fs-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name {
    font-weight: 600;
    font-size: var(--fs-md);
  }
  .name .sub {
    margin-left: 8px;
    font-weight: 400;
  }
  .big {
    margin-top: 6px;
    font-size: 30px;
    font-weight: 550;
    font-stretch: 82%;
    line-height: 1.1;
  }
  .big :global(small) {
    font-size: var(--fs-md);
  }
  dl {
    display: flex;
    gap: 18px;
    margin: auto 0 0;
  }
  dl div {
    display: flex;
    flex-direction: column;
  }
  dt {
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  dd {
    margin: 0;
    font-stretch: 88%;
  }
  .graph {
    min-width: 0;
    height: 100%;
  }
  .table {
    display: flex;
    flex-direction: column;
  }
  .row {
    display: grid;
    grid-template-columns: minmax(200px, 1fr) 132px 96px 96px 96px;
    align-items: center;
    column-gap: 4px;
  }
  .headrow {
    height: 38px;
    padding: 0 12px 0 16px;
    border-bottom: 1px solid var(--line);
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .r {
    text-align: right;
    justify-self: end;
  }
  .rows {
    flex: 1;
    overflow-y: auto;
    padding: 4px 4px 12px;
  }
  .device {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 40px;
    margin-top: 6px;
    padding: 0 8px;
    border-radius: var(--r-sm);
    text-align: left;
    color: var(--text);
    font-weight: 600;
    transition: background-color var(--d-micro);
  }
  .device:hover {
    background: var(--hover);
  }
  .chev {
    display: grid;
    place-items: center;
    width: 18px;
    color: var(--muted);
  }
  .chev :global(svg) {
    transition: transform var(--d-medium) var(--ease-spring);
  }
  .chev.open :global(svg) {
    transform: rotate(90deg);
  }
  .dicon {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 6px;
    color: var(--text-2);
    background: var(--raised);
    border: 1px solid var(--line-soft);
  }
  .dname {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dnote {
    color: var(--muted);
    font-weight: 400;
    font-size: var(--fs-sm);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dcount {
    margin-left: auto;
    color: var(--faint);
    font-weight: 500;
    font-size: var(--fs-sm);
  }
  .reading {
    position: relative;
    height: 34px;
    padding: 0 8px 0 58px;
    border-radius: var(--r-sm);
    content-visibility: auto;
    contain-intrinsic-size: auto 34px;
    transition: background-color var(--d-micro);
    animation: row-in var(--d-small) var(--ease-out) both;
  }
  @keyframes row-in {
    from {
      opacity: 0;
      transform: translateY(-3px);
    }
  }
  .reading:hover {
    background: var(--hover);
  }
  .reading.selected {
    background: var(--select);
  }
  .reading.selected::before {
    content: "";
    position: absolute;
    left: 0;
    top: 8px;
    bottom: 8px;
    width: 2px;
    border-radius: 2px;
    background: var(--text);
  }
  .label {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }
  .label .t {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    color: var(--faint);
    font-size: var(--fs-sm);
    white-space: nowrap;
  }
  .v {
    font-stretch: 88%;
    white-space: nowrap;
  }
  .v :global(small),
  .v small {
    margin-left: 3px;
    color: var(--faint);
    font-size: var(--fs-xs);
    font-stretch: 100%;
  }
  .v.warn {
    color: var(--warn);
  }
  .v.crit {
    color: var(--crit);
  }
  .cur {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 3px;
  }
  .meter {
    width: 64px;
    height: 2px;
    border-radius: 2px;
    background: var(--line-soft);
    overflow: hidden;
  }
  .meter i {
    display: block;
    height: 100%;
    background: var(--faint);
    transform-origin: left;
    transition: transform 240ms var(--ease-out);
  }
  .meter.warn i {
    background: var(--warn);
  }
  .meter.crit i {
    background: var(--crit);
  }
  :global(html.calm) .meter i {
    transition: none;
  }
  .empty {
    margin: 48px auto;
    text-align: center;
    color: var(--muted);
  }
</style>

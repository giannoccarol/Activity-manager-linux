<script lang="ts">
  import { app } from "../lib/state.svelte";
  import { bitsStr, ghz, num, pct0, rateStr, sizeStr, uptime } from "../lib/format";
  import type { Group } from "../lib/types";
  import Trace from "../components/Trace.svelte";
  import Icon from "../components/Icon.svelte";
  import AppIcon from "../components/AppIcon.svelte";
  import { pill } from "../lib/pill";

  type Item = { id: string; kind: "cpu" | "mem" | "disk" | "net" | "gpu"; name: string; sub: string; value: string; hist: string; max: number | "auto"; off?: boolean };

  let current = $state("cpu");
  let coreView = $state(false);
  let scale = $state(0);
  let scale2 = $state(0);
  const s = $derived(app.snap!);

  const items = $derived.by((): Item[] => {
    const out: Item[] = [
      { id: "cpu", kind: "cpu", name: "CPU", sub: ghz(s.cpu.freq), value: `${pct0(s.cpu.total)}%`, hist: "cpu", max: 100 },
      { id: "mem", kind: "mem", name: "Memoria", sub: `${sizeStr(s.mem.used)} di ${sizeStr(s.mem.total)}`, value: `${pct0(s.mem.pct)}%`, hist: "mem", max: 100 },
    ];
    s.disks.forEach((d, i) => out.push({ id: `disk:${d.name}`, kind: "disk", name: `Disco ${i}`, sub: d.kind, value: `${pct0(d.active)}%`, hist: `disk:${d.name}`, max: 100 }));
    s.nets.forEach((n) =>
      out.push({ id: `net:${n.name}`, kind: "net", name: n.kind, sub: n.ssid ?? n.name, value: n.connected ? bitsStr(n.recv + n.send) : "Non connessa", hist: `netR:${n.name}`, max: "auto", off: !n.connected }),
    );
    s.gpus.forEach((g) => out.push({ id: `gpu:${g.idx}`, kind: "gpu", name: `GPU ${g.idx}`, sub: g.name, value: g.suspended ? "Sospesa" : `${pct0(g.util)}%`, hist: `gpu:${g.idx}`, max: 100, off: g.suspended }));
    return out;
  });
  const cur = $derived(items.find((i) => i.id === current) ?? items[0]);

  function top(attr: "cpu" | "mem" | "disk" | "gpu"): [Group, string, number][] {
    const gs = [...s.groups].filter((g) => (g[attr] ?? 0) > 0).sort((a, b) => (b[attr] ?? 0) - (a[attr] ?? 0)).slice(0, 5);
    const total = attr === "cpu" ? Math.max(1, s.cpu.total) : attr === "mem" ? s.mem.total : attr === "disk" ? s.disks.reduce((a, d) => a + d.read + d.write, 0) || 1 : 100;
    return gs.map((g) => {
      const v = g[attr] ?? 0;
      const txt = attr === "mem" ? sizeStr(v) : attr === "disk" ? rateStr(v) : `${num(v, 1)}%`;
      return [g, txt, Math.min(1, v / total)];
    });
  }

  const disk = $derived(cur.kind === "disk" ? s.disks.find((d) => `disk:${d.name}` === cur.id) : undefined);
  const net = $derived(cur.kind === "net" ? s.nets.find((n) => `net:${n.name}` === cur.id) : undefined);
  const gpu = $derived(cur.kind === "gpu" ? s.gpus.find((g) => `gpu:${g.idx}` === cur.id) : undefined);
</script>

<div class="perf">
  <ul class="list" aria-label="Risorse" use:pill={{ selector: "button.active", axis: "y" }}>
    {#each items as it (it.id)}
      <li>
        <button class:active={it.id === cur.id} class:off={it.off} style="--c: var(--{it.kind})" onclick={() => (current = it.id)}>
          <span class="mini"><Trace samples={app.hist[it.hist] ?? []} now={s.t} max={it.max} floor={125000} color="var(--c)" graticule="none" /></span>
          <span class="txt">
            <span class="n">{it.name}</span>
            <span class="v">{it.value}</span>
            <span class="s">{it.sub}</span>
          </span>
        </button>
      </li>
    {/each}
  </ul>

  {#key cur.id}
  <section class="detail view-in" style="--c: var(--{cur.kind})">
    <header>
      <h2>{cur.name}</h2>
      <span class="device">
        {#if cur.kind === "cpu"}{s.cpu.model}{:else if disk}{disk.model}{:else if gpu}{gpu.name}{:else if net}{net.name}{:else}{sizeStr(s.mem.total, 0)} di RAM{/if}
      </span>
    </header>

    {#if cur.kind === "cpu"}
      <div class="tabs" role="tablist" use:pill>
        <button role="tab" aria-selected={!coreView} class:on={!coreView} onclick={() => (coreView = false)}>Complessivo</button>
        <button role="tab" aria-selected={coreView} class:on={coreView} onclick={() => (coreView = true)}>Per processore</button>
      </div>
    {/if}

    <div class="graph">
      <div class="scale"><span>{cur.kind === "net" ? bitsStr(scale) : "100%"}</span></div>
      {#if cur.kind === "cpu" && coreView}
        <div class="cores">
          {#each s.cpu.perCore as _, i}
            <div class="core"><Trace samples={app.hist[`core${i}`] ?? []} now={s.t} color="var(--c)" graticule="none" /></div>
          {/each}
        </div>
      {:else}
        <div class="big">
          <Trace
            samples={app.hist[cur.hist] ?? []}
            second={cur.kind === "cpu" ? app.hist.cpuKernel : net ? app.hist[`netS:${net.name}`] : undefined}
            names={cur.kind === "cpu" ? ["Totale", "Kernel"] : net ? ["Ricezione", "Invio"] : undefined}
            now={s.t}
            max={cur.max}
            floor={125000}
            color="var(--c)"
            graticule="full"
            onscale={(v) => (scale = v)}
            format={cur.kind === "net" ? bitsStr : (v) => `${num(v, 1)}%`} />
        </div>
      {/if}
      <div class="axis"><span>60 secondi</span><span>adesso</span></div>
      {#if disk}
        <div class="scale second"><span>Trasferimento</span><span>{rateStr(scale2)}</span></div>
        <div class="big small">
          <Trace samples={app.hist[`diskW:${disk.name}`] ?? []} second={app.hist[`diskR:${disk.name}`]} names={["Scrittura", "Lettura"]} now={s.t} max="auto" floor={1048576} color="var(--c)" graticule="full" format={rateStr} onscale={(v) => (scale2 = v)} />
        </div>
      {/if}
    </div>

    <dl class="readouts">
      {#if cur.kind === "cpu"}
        <div><dt>Utilizzo</dt><dd>{pct0(s.cpu.total)}%</dd></div>
        <div><dt>Velocità</dt><dd>{ghz(s.cpu.freq)}</dd></div>
        <div><dt>Temperatura</dt><dd class:hot={(s.cpu.temp ?? 0) >= 90}>{s.cpu.temp !== null ? `${pct0(s.cpu.temp)} °C` : "—"}</dd></div>
        <div><dt>Processi</dt><dd>{num(s.counts.procs, 0)}</dd></div>
        <div><dt>Thread</dt><dd>{num(s.counts.threads, 0)}</dd></div>
        <div><dt>Acceso da</dt><dd>{uptime(s.uptime)}</dd></div>
      {:else if cur.kind === "mem"}
        <div><dt>In uso</dt><dd>{sizeStr(s.mem.used)}</dd></div>
        <div><dt>Disponibile</dt><dd>{sizeStr(s.mem.available)}</dd></div>
        <div><dt>In cache</dt><dd>{sizeStr(s.mem.cached)}</dd></div>
        <div><dt>Swap</dt><dd>{sizeStr(s.mem.swapUsed)}</dd></div>
      {:else if disk}
        <div><dt>Attività</dt><dd class:hot={disk.active >= 90}>{pct0(disk.active)}%</dd></div>
        <div><dt>Scrittura</dt><dd>{rateStr(disk.write)}</dd></div>
        <div><dt>Lettura</dt><dd>{rateStr(disk.read)}</dd></div>
        <div><dt>Risposta media</dt><dd>{num(disk.resp, 0)} ms</dd></div>
        <div><dt>Capacità</dt><dd>{sizeStr(disk.capacity, 0)}</dd></div>
      {:else if net}
        <div><dt>Ricezione</dt><dd>{bitsStr(net.recv)}</dd></div>
        <div><dt>Invio</dt><dd>{bitsStr(net.send)}</dd></div>
        <div><dt>Collegamento</dt><dd>{net.link ? bitsStr(net.link) : "—"}</dd></div>
        <div><dt>Rete</dt><dd>{net.ssid ?? net.name}</dd></div>
      {:else if gpu}
        <div><dt>Utilizzo</dt><dd>{gpu.suspended ? "—" : `${pct0(gpu.util)}%`}</dd></div>
        <div><dt>Memoria</dt><dd>{sizeStr(gpu.memUsed)}{gpu.memTotal ? ` di ${sizeStr(gpu.memTotal, 0)}` : ""}</dd></div>
        <div><dt>Stato</dt><dd>{gpu.suspended ? "Sospesa per risparmiare energia" : "Attiva"}</dd></div>
      {/if}
    </dl>

    {#if cur.kind !== "net" && !(gpu && gpu.suspended)}
      {@const rows = top(cur.kind === "disk" ? "disk" : cur.kind === "mem" ? "mem" : cur.kind === "gpu" ? "gpu" : "cpu")}
      <section class="who">
        <h3>Chi sta usando {cur.kind === "cpu" ? "la CPU" : cur.kind === "mem" ? "la memoria" : cur.kind === "disk" ? "il disco" : "la GPU"}</h3>
        {#each rows as [g, txt, f] (g.key)}
          <div class="whorow">
            <span class="wi"><AppIcon src={g.icon} section={g.section} size={18} /></span>
            <span class="wn">{g.label}</span>
            <span class="wv">{txt}</span>
            <span class="wb"><i style="transform: scaleX({Math.max(0.01, f)})"></i></span>
          </div>
        {/each}
      </section>
    {/if}
  </section>
  {/key}
</div>

<style>
  .perf {
    display: grid;
    grid-template-columns: 252px minmax(0, 1fr);
    gap: 16px;
    height: 100%;
    min-height: 0;
    padding: 0 20px 18px;
  }
  .list {
    position: relative;
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .list button {
    position: relative;
    display: grid;
    grid-template-columns: 76px 1fr;
    gap: 12px;
    align-items: center;
    width: 100%;
    padding: 8px 10px;
    border-radius: var(--r-sm);
    text-align: left;
    z-index: 1;
    transition: background-color var(--d-micro);
  }
  .list button:hover {
    background: var(--hover);
  }
  .list button.active:hover {
    background: transparent;
  }
  .list > :global(.pill) {
    border-radius: var(--r-sm);
    background: var(--select);
    box-shadow: var(--sheen);
  }
  .list > :global(.pill)::before {
    content: "";
    position: absolute;
    left: 0;
    top: 14px;
    bottom: 14px;
    width: 2px;
    border-radius: 2px;
    background: var(--text);
  }
  .mini {
    display: block;
    height: 42px;
    border-radius: 5px;
    border: 1px solid var(--line);
    background: var(--panel);
    box-shadow: var(--sheen);
    overflow: hidden;
  }
  .off .mini {
    opacity: 0.5;
  }
  .txt {
    display: grid;
    grid-template-columns: 1fr auto;
    min-width: 0;
  }
  .n {
    font-weight: 500;
  }
  .v {
    color: var(--text-2);
    font-stretch: 88%;
  }
  .off .v {
    color: var(--faint);
  }
  .s {
    grid-column: 1 / -1;
    color: var(--muted);
    font-size: var(--fs-sm);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .detail {
    min-height: 0;
    overflow-y: auto;
    padding: 18px 22px 22px;
    background: var(--panel-bg);
    box-shadow: var(--panel-shadow);
    border: 1px solid var(--panel-edge);
    border-radius: var(--r-md);
  }
  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
  }
  h2 {
    margin: 0;
    font-size: 24px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .device {
    color: var(--text-2);
    font-size: var(--fs-md);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tabs {
    position: relative;
    display: inline-flex;
    gap: 2px;
    margin-top: 12px;
    padding: 3px;
    border-radius: var(--r-sm);
    background: var(--bg);
    border: 1px solid var(--line-soft);
  }
  .tabs button {
    position: relative;
    z-index: 1;
    height: 24px;
    padding: 0 10px;
    border-radius: 4px;
    color: var(--text-2);
    transition: color var(--d-small);
  }
  .tabs button.on {
    color: var(--text);
  }
  .tabs > :global(.pill) {
    border-radius: 4px;
    background: var(--raised);
    box-shadow: 0 0 0 1px var(--line);
  }
  .graph {
    margin-top: 14px;
  }
  .scale,
  .axis {
    display: flex;
    justify-content: flex-end;
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .scale.second {
    justify-content: space-between;
    margin-top: 16px;
  }
  .axis {
    justify-content: space-between;
    margin-top: 6px;
  }
  .big {
    height: 260px;
    margin-top: 6px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--bg);
  }
  .big.small {
    height: 110px;
  }
  .cores {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(92px, 1fr));
    gap: 6px;
    margin-top: 6px;
  }
  .core {
    height: 54px;
    border: 1px solid var(--line);
    border-radius: 5px;
    background: var(--bg);
    overflow: hidden;
  }
  .readouts {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 16px 24px;
    margin: 22px 0 0;
  }
  .readouts div {
    display: flex;
    flex-direction: column-reverse;
    gap: 2px;
  }
  dt {
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  dd {
    margin: 0;
    font-size: var(--fs-xl);
    font-weight: 500;
    font-stretch: 85%;
  }
  dd.hot {
    color: var(--warn);
  }
  .who {
    margin-top: 26px;
    padding-top: 18px;
    border-top: 1px solid var(--line-soft);
  }
  h3 {
    margin: 0 0 10px;
    font-size: var(--fs-base);
    font-weight: 600;
    color: var(--text-2);
  }
  .whorow {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr) auto;
    grid-template-rows: auto 3px;
    column-gap: 10px;
    row-gap: 5px;
    align-items: center;
    padding: 7px 0;
  }
  .wi {
    display: grid;
    grid-row: 1 / span 2;
  }
  .wn {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .wv {
    color: var(--text-2);
    font-stretch: 88%;
  }
  .wb {
    grid-column: 2 / -1;
    height: 3px;
    border-radius: 2px;
    background: var(--line-soft);
    overflow: hidden;
  }
  .wb i {
    display: block;
    height: 100%;
    background: var(--c);
    transform-origin: left;
    transition: transform 240ms var(--ease-out);
  }
</style>

<script lang="ts">
  import { app } from "../lib/state.svelte";
  import { act } from "../lib/api";
  import { meter, num, rate } from "../lib/format";
  import type { Group, Proc, Section, Status } from "../lib/types";
  import Icon from "./Icon.svelte";
  import AppIcon from "./AppIcon.svelte";
  import ContextMenu, { type MenuItem } from "./ContextMenu.svelte";
  import { flip } from "svelte/animate";
  import { away, easeOut, fadeq, flags, flipMs, rise } from "../lib/motion.svelte";

  let { filter = "all" }: { filter?: Section | "all" } = $props();

  type Key = "name" | "status" | "cpu" | "mem" | "disk" | "gpu";
  let sortKey = $state<Key>("cpu");
  let dir = $state<1 | -1>(-1);
  let open = $state<Record<string, boolean>>({});
  let menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);
  let hover = $state<string | null>(null);
  const memTotal = $derived(app.snap?.mem.total ?? 1);

  const SECTIONS: [Section, string][] = [
    ["app", "App"],
    ["ctr", "Container"],
    ["bg", "In background"],
    ["sys", "Di sistema"],
  ];
  const STATUS: Record<Status, string> = { running: "In esecuzione", stopped: "Sospeso", efficiency: "Efficienza", paused: "In pausa" };
  const MiB = 1024 * 1024;

  function sortBy(k: Key) {
    if (sortKey === k) dir = dir === 1 ? -1 : 1;
    else {
      sortKey = k;
      dir = k === "name" || k === "status" ? 1 : -1;
    }
  }

  // valori arrotondati per ordinare: due processi quasi alla pari non si scambiano a ogni secondo
  // (meno righe che si muovono, riordino animato solo per i cambi veri)
  function val(x: Group | Proc, k: Key): number | string {
    if (k === "name") return ("label" in x ? x.label : x.name).toLowerCase();
    if (k === "status") return x.status;
    if (k === "disk") return Math.round((x.disk ?? -1) / 65536);
    if (k === "mem") return Math.round(x.mem / 4194304);
    return Math.round(x[k] * 2) / 2;
  }
  function cmp(a: Group | Proc, b: Group | Proc) {
    const va = val(a, sortKey);
    const vb = val(b, sortKey);
    if (va !== vb) return (va < vb ? -1 : 1) * dir;
    const na = "label" in a ? a.label : a.name;
    const nb = "label" in b ? b.label : b.name;
    return na < nb ? -1 : na > nb ? 1 : 0;
  }
  // barretta a gradini del 2,5%: le variazioni invisibili non fanno partire animazioni
  const q = (v: number) => Math.round(v * flags.meterSteps) / flags.meterSteps;

  const needle = $derived(app.search.trim().toLowerCase());
  // gruppi mostrati solo per alcuni processi figli: si aprono da soli per far vedere quali
  let narrowed = new Set<string>();
  function matches(g: Group): Group | null {
    if (!needle) return g;
    if (g.label.toLowerCase().includes(needle) || g.image?.toLowerCase().includes(needle)) return g;
    // nella riga di comando si cerca dal terzo carattere: prima corrisponderebbe quasi tutto
    const inCmd = needle.length >= 3;
    const procs = g.procs.filter(
      (p) => p.name.toLowerCase().includes(needle) || String(p.pid) === needle || (inCmd && p.cmd.toLowerCase().includes(needle)),
    );
    if (!procs.length) return null;
    narrowed.add(g.key);
    return { ...g, procs };
  }

  // molte righe che compaiono o spariscono insieme (ricerca, filtro): niente animazione per riga
  let shownKeys = new Set<string>();
  let bulk = false;
  const sections = $derived.by(() => {
    narrowed = new Set();
    const groups = (app.snap?.groups ?? []).map(matches).filter((g): g is Group => !!g);
    const keys = new Set(groups.map((g) => g.key));
    let changed = 0;
    for (const k of keys) if (!shownKeys.has(k)) changed++;
    for (const k of shownKeys) if (!keys.has(k)) changed++;
    bulk = changed > 6;
    shownKeys = keys;
    return SECTIONS.filter(([s]) => filter === "all" || filter === s)
      .map(([s, title]) => ({ s, title, groups: groups.filter((g) => g.section === s).sort(cmp) }))
      .filter((x) => x.groups.length);
  });

  function procKey(p: Proc) {
    return `p:${p.pid}:${p.ctime}`;
  }

  function terminate(target: Group | Proc) {
    if ("procs" in target) act(target.section === "ctr" ? "container" : "terminate", target.section === "ctr" ? { verb: "stop", key: target.key } : { key: target.key });
    else act("signal", { pid: target.pid, ctime: target.ctime, sig: "TERM" });
  }

  function openMenu(e: MouseEvent, target: Group | Proc) {
    e.preventDefault();
    app.selected = "procs" in target ? target.key : procKey(target);
    const isGroup = "procs" in target;
    const ctr = isGroup && target.section === "ctr";
    const key = isGroup ? target.key : null;
    const items: MenuItem[] = [];
    if (isGroup && target.procs.length > 1) items.push({ label: open[target.key] ? "Comprimi" : "Espandi", run: () => (open[target.key] = !open[target.key]) });
    if (ctr) {
      items.push(
        { label: target.status === "paused" ? "Riprendi" : "Metti in pausa", icon: target.status === "paused" ? "play" : "pause", run: () => act("container", { verb: target.status === "paused" ? "unpause" : "pause", key }) },
        { label: "Riavvia", icon: "restart", run: () => act("container", { verb: "restart", key }) },
        { label: "Mostra log", icon: "terminal", run: () => act("container", { verb: "logs", key }) },
        { label: "Apri una shell", icon: "terminal", run: () => act("container", { verb: "shell", key }) },
        { sep: true },
        { label: "Arresta il container", icon: "stop", danger: true, run: () => act("container", { verb: "stop", key }) },
      );
    } else {
      items.push(
        { label: "Termina attività", icon: "x", run: () => terminate(target) },
        { label: "Termina forzatamente", danger: true, run: () => (isGroup ? act("terminate", { key, force: true }) : act("signal", { pid: target.pid, ctime: target.ctime, sig: "KILL" })) },
        { label: target.status === "stopped" ? "Riprendi" : "Sospendi", icon: target.status === "stopped" ? "play" : "pause", run: () => act("suspend", { key, pid: isGroup ? null : target.pid, resume: target.status === "stopped" }) },
        { label: "Modalità efficienza", icon: "leaf", checked: target.status === "efficiency", run: () => act("efficiency", { key, pid: isGroup ? null : target.pid, on: target.status !== "efficiency" }) },
        { sep: true },
        { label: "Apri percorso file", icon: "folder", run: () => act("open-location", { key, pid: isGroup ? null : target.pid }) },
        { label: "Copia riga di comando", icon: "copy", run: () => navigator.clipboard?.writeText(isGroup ? target.procs[0]?.cmd ?? "" : target.cmd) },
        { label: "Proprietà", icon: "info", run: () => act("properties", { key, pid: isGroup ? target.procs[0]?.pid : target.pid }) },
      );
    }
    menu = { x: e.clientX, y: e.clientY, items };
  }
</script>

<!-- una cella numerica è un solo elemento: la barretta è disegnata in CSS da --p -->
{#snippet numcell(v: number | null, kind: "cpu" | "mem" | "disk" | "gpu")}
  {#if v === null}
    <span class="num na" title="Non leggibile senza privilegi">—</span>
  {:else}
    <span class="num {kind}" style:--p={q(kind === "mem" ? meter(v, memTotal) : kind === "disk" ? meter(v, 200 * MiB) : meter(v, 100))}>
      {#if kind === "mem"}{num(v / MiB, 1)}<small>MB</small>
      {:else if kind === "disk"}{@const r = rate(v)}{r[0]}<small>{r[1]}</small>
      {:else}{v < 0.05 ? "0" : num(v, 1)}<small>%</small>{/if}
    </span>
  {/if}
{/snippet}

{#snippet tools(target: Group | Proc, k: string, label: string)}
  {#if hover === k || app.selected === k}
    <button class="kill" title="Termina" aria-label="Termina {label}" onclick={(e) => { e.stopPropagation(); terminate(target); }}><Icon name="x" size={14} stroke={1.8} /></button>
    <button class="more" title="Altre azioni" aria-label="Altre azioni" onclick={(e) => { e.stopPropagation(); openMenu(e, target); }}><Icon name="kebab" size={16} stroke={2.4} /></button>
  {/if}
{/snippet}

{#snippet status(st: Status)}
  <span class="status {st}"><i></i>{STATUS[st]}</span>
{/snippet}

<div class="table" role="treegrid" aria-label="Processi">
  <div class="head" role="row">
    {#each [["name", "Nome"], ["status", "Stato"], ["cpu", "CPU"], ["mem", "Memoria"], ["disk", "Disco"], ["gpu", "GPU"]] as [k, label]}
      <button class="h {k}" class:sorted={sortKey === k} onclick={() => sortBy(k as Key)} role="columnheader" aria-sort={sortKey === k ? (dir === 1 ? "ascending" : "descending") : "none"}>
        {label}
        {#if sortKey === k}<Icon name={dir === 1 ? "chev-up" : "chev-down"} size={12} stroke={2} />{/if}
      </button>
    {/each}
    <span class="h engine">Motore GPU</span>
    <span class="h tools"></span>
  </div>

  <div class="rows" data-scroll>
    {#each sections as sec (sec.s)}
      {#if filter === "all"}
        <div class="section" role="row"><span>{sec.title}</span><span class="count">{sec.groups.length}</span></div>
      {/if}
      {#each sec.groups as g (g.key)}
        {@const multi = g.procs.length > 1}
        {@const isOpen = open[g.key] || (!!needle && multi && narrowed.has(g.key))}
        <!-- ogni gruppo scivola nella nuova posizione quando cambia l'ordine -->
        <div class="grp" animate:flip={{ duration: flipMs(), easing: easeOut }} in:fadeq={{ duration: bulk ? 0 : 200 }} out:away={{ duration: bulk ? 0 : 200 }}>
        <div
          class="row group"
          class:selected={app.selected === g.key}
          role="row"
          tabindex="0"
          onpointerenter={() => (hover = g.key)}
          onpointerleave={() => hover === g.key && (hover = null)}
          aria-expanded={multi ? isOpen : undefined}
          onclick={() => (app.selected = g.key)}
          ondblclick={() => multi && (open[g.key] = !open[g.key])}
          oncontextmenu={(e) => openMenu(e, g)}
          onkeydown={(e) => {
            if (e.key === "Delete") terminate(g);
            if (e.key === "ArrowRight" && multi) open[g.key] = true;
            if (e.key === "ArrowLeft") open[g.key] = false;
          }}>
          <span class="name">
            {#if multi}
              <button class="chev" class:open={isOpen} aria-label={isOpen ? "Comprimi" : "Espandi"} onclick={(e) => { e.stopPropagation(); open[g.key] = !open[g.key]; }}>
                <Icon name="chev-right" size={14} stroke={2} />
              </button>
            {:else}<span class="chev-space"></span>{/if}
            <AppIcon src={g.icon} section={g.section} />
            <span class="label">{g.label}</span>
            {#if g.image}<span class="sub">{g.image}</span>{/if}
            {#if multi}<span class="sub">{g.procs.length}</span>{/if}
          </span>
          {@render status(g.status)}
          {@render numcell(g.cpu, "cpu")}
          {@render numcell(g.mem, "mem")}
          {@render numcell(g.disk, "disk")}
          {@render numcell(g.gpu, "gpu")}
          <span class="engine">{g.gpuEngine}</span>
          <span class="tools">{@render tools(g, g.key, g.label)}</span>
        </div>
        {#if multi && isOpen}
          {#each [...g.procs].sort(cmp) as p, i (procKey(p))}
            {@const pk = procKey(p)}
            <div class="row child" class:selected={app.selected === pk} role="row" tabindex="-1" onclick={() => (app.selected = pk)} oncontextmenu={(e) => openMenu(e, p)} onpointerenter={() => (hover = pk)} onpointerleave={() => hover === pk && (hover = null)} in:rise={{ y: -6, duration: 200, delay: Math.min(i, 12) * 16 }} out:fadeq={{ duration: 90 }}>
              <span class="name">
                <span class="chev-space"></span>
                <span class="cicon"><AppIcon src={p.icon ?? g.icon} section={g.section} size={16} /></span>
                <span class="label">{p.name}</span><span class="sub">{p.pid}</span>
              </span>
              {@render status(p.status)}
              {@render numcell(p.cpu, "cpu")}
              {@render numcell(p.mem, "mem")}
              {@render numcell(p.disk, "disk")}
              {@render numcell(p.gpu, "gpu")}
              <span class="engine">{p.gpuEngine}</span>
              <span class="tools">{@render tools(p, pk, p.name)}</span>
            </div>
          {/each}
        {/if}
        </div>
      {/each}
    {/each}
    {#if !sections.length}
      <p class="empty">Nessun processo corrisponde a «{app.search}». Prova con il nome dell'app, il PID o una parte della riga di comando.</p>
    {/if}
  </div>
</div>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} items={menu.items} onclose={() => (menu = null)} />
{/if}

<style>
  .table {
    --cols: minmax(220px, 1fr) 132px 92px 112px 104px 80px 112px 64px;
    display: flex;
    flex-direction: column;
    min-height: 0;
    height: 100%;
    container-type: inline-size;
  }
  .head,
  .row {
    display: grid;
    grid-template-columns: var(--cols);
    align-items: center;
    column-gap: 4px;
  }
  @container (max-width: 980px) {
    .head,
    .row {
      grid-template-columns: minmax(200px, 1fr) 120px 88px 108px 100px 76px 0 64px;
    }
    .engine {
      visibility: hidden;
    }
  }
  .head {
    height: 38px;
    padding: 0 8px;
    border-bottom: 1px solid var(--line);
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .h {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 100%;
    padding: 0 8px;
    color: inherit;
  }
  .h:hover,
  .h.sorted {
    color: var(--text);
  }
  .h.cpu,
  .h.mem,
  .h.disk,
  .h.gpu {
    justify-content: flex-end;
  }
  .rows {
    overflow-y: auto;
    flex: 1;
    padding: 4px 4px 12px;
  }
  .section {
    display: flex;
    gap: 8px;
    align-items: baseline;
    padding: 14px 12px 6px;
    font-weight: 600;
    color: var(--text-2);
  }
  .section .count {
    color: var(--faint);
    font-weight: 500;
  }
  .row {
    position: relative;
    height: 36px;
    /* righe fuori schermo: niente impaginazione né disegno */
    content-visibility: auto;
    contain-intrinsic-size: auto 36px;
    contain: layout style;
    padding: 0 4px;
    border-radius: var(--r-sm);
    outline-offset: -2px;
    transition: background-color 120ms;
  }
  .row:hover {
    background: var(--hover);
  }
  .row.selected {
    background: var(--select);
  }
  .row.selected::before {
    content: "";
    position: absolute;
    left: 0;
    top: 9px;
    bottom: 9px;
    width: 2px;
    border-radius: 2px;
    background: var(--text);
    animation: bar-in var(--d-small) var(--ease-spring) both;
  }
  @keyframes bar-in {
    from {
      transform: scaleY(0.2);
      opacity: 0;
    }
  }
  .name {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding-left: 4px;
  }
  .chev,
  .cicon {
    display: grid;
    place-items: center;
    width: 20px;
    margin-left: 22px;
    flex: none;
  }
  .chev-space {
    flex: none;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    color: var(--muted);
    border-radius: 4px;
  }
  .chev:hover {
    color: var(--text);
    background: var(--hover);
  }
  .chev :global(svg) {
    transition: transform var(--d-medium) var(--ease-spring);
  }
  .chev.open :global(svg) {
    transform: rotate(90deg);
  }
  .name img {
    width: 20px;
    height: 20px;
    flex: none;
    object-fit: contain;
  }
  .glyph {
    flex: none;
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 5px;
    color: var(--muted);
    background: var(--raised);
    border: 1px solid var(--line-soft);
  }
  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .child .label {
    color: var(--text-2);
  }
  .sub {
    color: var(--faint);
    font-size: var(--fs-sm);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex-shrink: 1;
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 0 8px;
    color: var(--text-2);
    font-size: var(--fs-sm);
    white-space: nowrap;
  }
  .status i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ok);
  }
  .status.stopped i,
  .status.paused i {
    background: var(--warn);
  }
  .status.efficiency i {
    background: var(--mem);
  }
  .num {
    position: relative;
    display: flex;
    align-items: baseline;
    justify-content: flex-end;
    align-self: stretch;
    padding: 9px 8px 0;
    font-stretch: 88%;
  }
  .num small {
    margin-left: 3px;
    color: var(--faint);
    font-size: var(--fs-xs);
    font-stretch: 100%;
  }
  .num.na {
    color: var(--faint);
  }
  /* barretta: binario e riempimento, senza elementi in più */
  .num:not(.na)::before,
  .num:not(.na)::after {
    content: "";
    position: absolute;
    right: 8px;
    bottom: 8px;
    width: min(64px, calc(100% - 16px));
    height: 2px;
    border-radius: 2px;
    background: var(--line-soft);
  }
  .num:not(.na)::after {
    background: var(--c);
    transform: scaleX(var(--p, 0));
    transform-origin: right;
    transition: transform 240ms var(--ease-out);
  }
  :global(html.calm) .num::after {
    transition: none;
  }
  .num.cpu {
    --c: var(--cpu);
  }
  .num.mem {
    --c: var(--mem);
  }
  .num.disk {
    --c: var(--disk);
  }
  .num.gpu {
    --c: var(--gpu);
  }
  .engine {
    padding: 0 8px;
    color: var(--muted);
    font-size: var(--fs-sm);
    white-space: nowrap;
  }
  .tools {
    display: flex;
    justify-content: flex-end;
    gap: 2px;
    padding-right: 4px;
  }
  .tools button {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: var(--r-sm);
    color: var(--muted);
    animation: tool-in var(--d-small) var(--ease-out) both;
    transition:
      background-color var(--d-micro),
      color var(--d-micro),
      transform var(--d-micro) var(--ease-out);
  }
  .tools button + button {
    animation-delay: 30ms;
  }
  .tools button:active {
    transform: scale(0.9);
  }
  @keyframes tool-in {
    from {
      opacity: 0;
      transform: translateX(6px);
    }
  }
  .tools .kill:hover {
    color: #fff;
    background: var(--crit);
  }
  .tools .more:hover {
    color: var(--text);
    background: var(--hover);
  }
  .empty {
    margin: 48px auto;
    max-width: 52ch;
    text-align: center;
    color: var(--muted);
  }
</style>

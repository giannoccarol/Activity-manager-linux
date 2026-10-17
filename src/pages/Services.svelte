<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, act } from "../lib/api";
  import { ui } from "../lib/ui.svelte";
  import { app } from "../lib/state.svelte";
  import { num, sizeStr } from "../lib/format";
  import { mockServices } from "../lib/mockpages";
  import type { Col } from "../lib/table";
  import type { Group, Service } from "../lib/types";
  import DataTable from "../components/DataTable.svelte";
  import { pill } from "../lib/pill";
  import Icon from "../components/Icon.svelte";
  import ContextMenu, { type MenuItem } from "../components/ContextMenu.svelte";

  // System and user systemd services: start, stop, autostart.
  type F = "all" | "running" | "stopped" | "failed";
  const FILTERS: [F, string][] = [
    ["all", "Tutti"],
    ["running", "In esecuzione"],
    ["stopped", "Arrestati"],
    ["failed", "Con errori"],
  ];
  const BOOT: Record<string, string> = {
    enabled: "Automatico",
    "enabled-runtime": "Automatico",
    disabled: "Manuale",
    static: "Su richiesta",
    indirect: "Su richiesta",
    generated: "Generato",
    masked: "Disattivato",
    "masked-runtime": "Disattivato",
    alias: "Alias",
    linked: "Collegato",
    transient: "Temporaneo",
  };

  let list = $state<Service[]>([]);
  let loaded = $state(false);
  let filter = $state<F>("all");
  let selected = $state<string | null>(null);
  let sortKey = $state("name");
  let sortDir = $state<1 | -1>(1);
  let busy = $state(false);
  let menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);

  async function load() {
    try {
      list = await invoke<Service[]>("services_list", {}, mockServices);
    } catch (e) {
      ui.toast(String(e), "crit");
    }
    loaded = true;
  }
  onMount(() => {
    load();
    const t = setInterval(load, 10000);
    return () => clearInterval(t);
  });

  async function run(s: Service, verb: string) {
    busy = true;
    try {
      const msg = await invoke<string>("service_action", { unit: s.unit, scope: s.scope, verb }, () => "ok");
      ui.toast(msg);
    } catch (e) {
      ui.toast(String(e), "crit");
    }
    busy = false;
    load();
  }

  const id = (s: Service) => `${s.scope}:${s.unit}`;
  function runState(s: Service): F {
    return s.active === "failed" ? "failed" : s.active === "active" || s.active === "activating" || s.active === "reloading" ? "running" : "stopped";
  }
  // the service's processes from the snapshot, for memory and CPU
  const byUnit = $derived.by(() => {
    const m = new Map<string, Group>();
    for (const g of app.snap?.groups ?? []) if (g.unit) m.set(`${g.section === "sys" ? "system" : "user"}:${g.unit}`, g);
    return m;
  });
  const needle = $derived(app.search.trim().toLowerCase());
  const shown = $derived(
    list.filter((s) => (filter === "all" || runState(s) === filter) && (!needle || s.unit.toLowerCase().includes(needle) || s.description.toLowerCase().includes(needle))),
  );
  const count = (f: F) => (f === "all" ? list.length : list.filter((s) => runState(s) === f).length);
  const sel = $derived(list.find((s) => id(s) === selected) ?? null);

  function items(s: Service): MenuItem[] {
    const st = runState(s);
    const g = byUnit.get(id(s));
    const auto = s.state === "enabled" || s.state === "enabled-runtime";
    const masked = s.state.startsWith("masked");
    const log = `journalctl ${s.scope === "user" ? "--user " : ""}-u '${s.unit}' -n 300 -f`;
    return [
      { label: "Avvia", icon: "play", disabled: st === "running" || masked, run: () => run(s, "start") },
      { label: "Arresta", icon: "stop", disabled: st !== "running", run: () => run(s, "stop") },
      { label: "Riavvia", icon: "restart", disabled: masked, run: () => run(s, "restart") },
      { sep: true },
      { label: "Avvio automatico", checked: auto, disabled: masked || s.state === "static" || s.state === "indirect", run: () => run(s, auto ? "disable" : "enable") },
      { label: masked ? "Riattiva il servizio" : "Disattiva del tutto", danger: !masked, run: () => run(s, masked ? "unmask" : "mask") },
      { sep: true },
      { label: "Mostra il registro", icon: "terminal", run: () => act("run", { cmd: `konsole --hold -e sh -c "${log}"` }) },
      {
        label: "Vai ai processi",
        icon: "processes",
        disabled: !g,
        run: () => {
          if (!g) return;
          app.selected = g.key;
          app.search = "";
          app.page = "processes";
        },
      },
    ];
  }
</script>

{#snippet name(s: Service)}<span class="t">{s.unit.replace(/\.service$/, "")}</span>{/snippet}
{#snippet desc(s: Service)}<span class="soft t" title={s.description}>{s.description}</span>{/snippet}
{#snippet status(s: Service)}
  {@const st = runState(s)}
  <span class="dot-status" class:ok={st === "running"} class:crit={st === "failed"}><i></i>{st === "running" ? (s.active === "activating" ? "In avvio" : "In esecuzione") : st === "failed" ? "Errore" : "Arrestato"}</span>
{/snippet}
{#snippet boot(s: Service)}<span class="soft">{BOOT[s.state] ?? s.state}</span>{/snippet}
{#snippet mem(s: Service)}
  {@const g = byUnit.get(id(s))}
  {#if g}{sizeStr(g.mem)}{:else}<span class="faint">—</span>{/if}
{/snippet}
{#snippet cpu(s: Service)}
  {@const g = byUnit.get(id(s))}
  {#if g}{g.cpu < 0.05 ? "0" : num(g.cpu, 1)}<small>%</small>{:else}<span class="faint">—</span>{/if}
{/snippet}

<div class="pg">
  <div class="pg-bar">
    <div class="pg-seg" role="tablist" aria-label="Filtra i servizi" use:pill>
      {#each FILTERS as [f, label]}
        <button role="tab" aria-selected={filter === f} class:on={filter === f} onclick={() => (filter = f)}>{label}<span>{count(f)}</span></button>
      {/each}
    </div>
    <div class="pg-actions">
      <button class="btn" disabled={!sel || busy || runState(sel) === "running"} onclick={() => sel && run(sel, "start")}><Icon name="play" size={13} />Avvia</button>
      <button class="btn" disabled={!sel || busy || runState(sel) !== "running"} onclick={() => sel && run(sel, "stop")}><Icon name="stop" size={13} />Arresta</button>
      <button class="btn" disabled={!sel || busy} onclick={() => sel && run(sel, "restart")}><Icon name="restart" size={13} />Riavvia</button>
    </div>
  </div>
  <div class="pg-panel">
    <DataTable
      label="Servizi"
      rows={shown}
      rowKey={id}
      bind:selected
      bind:sortKey
      bind:sortDir
      group={(s) => (s.scope === "user" ? ["Servizi dell'utente", 1] : ["Servizi di sistema", 2])}
      empty={loaded ? "Nessun servizio in questo elenco" : "Leggo i servizi…"}
      onmenu={(e, s) => (menu = { x: e.clientX, y: e.clientY, items: items(s) })}
      cols={[
        { key: "name", label: "Nome", width: "minmax(180px, 1fr)", sort: (s) => s.unit.toLowerCase(), cell: name },
        { key: "desc", label: "Descrizione", width: "minmax(220px, 1.5fr)", sort: (s) => s.description.toLowerCase(), cell: desc },
        { key: "status", label: "Stato", width: "136px", sort: (s) => runState(s), cell: status },
        { key: "boot", label: "Avvio", width: "120px", sort: (s) => BOOT[s.state] ?? s.state, cell: boot },
        { key: "cpu", label: "CPU", width: "72px", align: "right", sort: (s) => byUnit.get(id(s))?.cpu ?? -1, cell: cpu },
        { key: "mem", label: "Memoria", width: "96px", align: "right", sort: (s) => byUnit.get(id(s))?.mem ?? -1, cell: mem },
      ] satisfies Col<Service>[]}
    />
  </div>
</div>
{#if menu}<ContextMenu {...menu} onclose={() => (menu = null)} />{/if}

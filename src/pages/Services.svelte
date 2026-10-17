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
  import { t } from "../lib/i18n.svelte";

  // System and user systemd services: start, stop, autostart.
  type F = "all" | "running" | "stopped" | "failed";
  // labels: services.filters.<filter>
  const FILTERS: F[] = ["all", "running", "stopped", "failed"];
  // systemd unit file state -> services.boot.<key>
  const BOOT: Record<string, string> = {
    enabled: "automatic",
    "enabled-runtime": "automatic",
    disabled: "manual",
    static: "onDemand",
    indirect: "onDemand",
    generated: "generated",
    masked: "masked",
    "masked-runtime": "masked",
    alias: "alias",
    linked: "linked",
    transient: "transient",
  };
  const boot = (s: Service) => (BOOT[s.state] ? t(`services.boot.${BOOT[s.state]}`) : s.state);

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
      { label: t("services.start"), icon: "play", disabled: st === "running" || masked, run: () => run(s, "start") },
      { label: t("services.stop"), icon: "stop", disabled: st !== "running", run: () => run(s, "stop") },
      { label: t("services.restart"), icon: "restart", disabled: masked, run: () => run(s, "restart") },
      { sep: true },
      { label: t("services.menu.autostart"), checked: auto, disabled: masked || s.state === "static" || s.state === "indirect", run: () => run(s, auto ? "disable" : "enable") },
      { label: t(masked ? "services.menu.unmask" : "services.menu.mask"), danger: !masked, run: () => run(s, masked ? "unmask" : "mask") },
      { sep: true },
      { label: t("services.menu.showLog"), icon: "terminal", run: () => act("run", { cmd: `konsole --hold -e sh -c "${log}"` }) },
      {
        label: t("services.menu.goToProcesses"),
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
  <span class="dot-status" class:ok={st === "running"} class:crit={st === "failed"}><i></i>{t(st === "running" ? (s.active === "activating" ? "services.state.starting" : "services.state.running") : st === "failed" ? "services.state.failed" : "services.state.stopped")}</span>
{/snippet}
{#snippet bootCell(s: Service)}<span class="soft">{boot(s)}</span>{/snippet}
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
    <div class="pg-seg" role="tablist" aria-label={t("services.filterLabel")} use:pill>
      {#each FILTERS as f}
        <button role="tab" aria-selected={filter === f} class:on={filter === f} onclick={() => (filter = f)}>{t(`services.filters.${f}`)}<span>{count(f)}</span></button>
      {/each}
    </div>
    <div class="pg-actions">
      <button class="btn" disabled={!sel || busy || runState(sel) === "running"} onclick={() => sel && run(sel, "start")}><Icon name="play" size={13} />{t("services.start")}</button>
      <button class="btn" disabled={!sel || busy || runState(sel) !== "running"} onclick={() => sel && run(sel, "stop")}><Icon name="stop" size={13} />{t("services.stop")}</button>
      <button class="btn" disabled={!sel || busy} onclick={() => sel && run(sel, "restart")}><Icon name="restart" size={13} />{t("services.restart")}</button>
    </div>
  </div>
  <div class="pg-panel">
    <DataTable
      label={t("nav.services")}
      rows={shown}
      rowKey={id}
      bind:selected
      bind:sortKey
      bind:sortDir
      group={(s) => (s.scope === "user" ? [t("services.groups.user"), 1] : [t("services.groups.system"), 2])}
      empty={loaded ? t("services.empty") : t("services.loading")}
      onmenu={(e, s) => (menu = { x: e.clientX, y: e.clientY, items: items(s) })}
      cols={[
        { key: "name", label: t("common.name"), width: "minmax(180px, 1fr)", sort: (s) => s.unit.toLowerCase(), cell: name },
        { key: "desc", label: t("services.description"), width: "minmax(220px, 1.5fr)", sort: (s) => s.description.toLowerCase(), cell: desc },
        { key: "status", label: t("common.status"), width: "136px", sort: (s) => runState(s), cell: status },
        { key: "boot", label: t("services.startup"), width: "120px", sort: boot, cell: bootCell },
        { key: "cpu", label: "CPU", width: "72px", align: "right", sort: (s) => byUnit.get(id(s))?.cpu ?? -1, cell: cpu },
        { key: "mem", label: t("common.memory"), width: "96px", align: "right", sort: (s) => byUnit.get(id(s))?.mem ?? -1, cell: mem },
      ] satisfies Col<Service>[]}
    />
  </div>
</div>
{#if menu}<ContextMenu {...menu} onclose={() => (menu = null)} />{/if}

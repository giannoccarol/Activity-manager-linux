<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "../lib/api";
  import { ui } from "../lib/ui.svelte";
  import { ago, dur, sizeStr } from "../lib/format";
  import { mockHistory } from "../lib/mockpages";
  import type { Col } from "../lib/table";
  import type { HistoryRow, Section } from "../lib/types";
  import DataTable from "../components/DataTable.svelte";
  import { pill } from "../lib/pill";
  import AppIcon from "../components/AppIcon.svelte";
  import Switch from "../components/Switch.svelte";
  import Dialog from "../components/Dialog.svelte";
  import { app } from "../lib/state.svelte";
  import { t } from "../lib/i18n.svelte";

  // How much each app has used over time, from the history database.
  const RANGES = [1, 7, 30];
  let days = $state(1);
  let onlyApps = $state(true);
  let rows = $state<HistoryRow[]>([]);
  let loaded = $state(false);
  let selected = $state<string | null>(null);
  let sortKey = $state("cpu");
  let sortDir = $state<1 | -1>(-1);
  let confirm = $state(false);

  async function load() {
    try {
      rows = await invoke<HistoryRow[]>("history", { days }, mockHistory);
    } catch (e) {
      ui.toast(String(e), "crit");
    }
    loaded = true;
  }
  onMount(() => {
    const t = setInterval(load, 15000);
    return () => clearInterval(t);
  });
  $effect(() => {
    days;
    load();
  });

  async function clear() {
    confirm = false;
    try {
      await invoke("history_clear", {}, () => null);
      rows = [];
      ui.toast(t("toasts.historyCleared"));
    } catch (e) {
      ui.toast(String(e), "crit");
    }
  }

  // each container restart gets a new name: rows with the same image are merged into one
  type Row = HistoryRow & { runs: number };
  const merged = $derived.by(() => {
    const out = new Map<string, Row>();
    for (const r of rows) {
      const [, img] = split(r.app);
      const k = r.kind === "ctr" && img ? `ctr:${img}` : r.app;
      const m = out.get(k);
      if (!m) out.set(k, { ...r, app: r.kind === "ctr" && img ? img : r.app, runs: 1 });
      else {
        m.cpuS += r.cpuS;
        m.diskB += r.diskB;
        m.gpuS += r.gpuS;
        m.memPeak = Math.max(m.memPeak, r.memPeak);
        m.last = Math.max(m.last, r.last);
        m.runs++;
      }
    }
    return [...out.values()];
  });
  const needle = $derived(app.search.trim().toLowerCase());
  const shown = $derived(merged.filter((r) => (!onlyApps || r.kind === "app" || r.kind === "ctr") && (!needle || r.app.toLowerCase().includes(needle))));
  const totals = $derived(shown.reduce((a, r) => a + r.cpuS, 0));
  // containers are stored as "name · image"
  function split(s: string) {
    const i = s.indexOf("·");
    return i < 0 ? [s, ""] : [s.slice(0, i).trim(), s.slice(i + 1).trim()];
  }
</script>

{#snippet name(r: Row)}
  {@const [n, img] = split(r.app)}
  <AppIcon src={r.icon} section={r.kind as Section} size={18} />
  <span class="t">{n}</span>
  {#if img}<span class="faint t">{img}</span>{/if}
  {#if r.runs > 1}<span class="faint">{t("history.runs", { count: r.runs })}</span>{/if}
{/snippet}
{#snippet cpu(r: Row)}
  <span class="share"><i style="transform: scaleX({totals ? Math.max(0.02, r.cpuS / totals) : 0})"></i></span><span class="dur">{dur(r.cpuS)}</span>
{/snippet}
{#snippet disk(r: Row)}{sizeStr(r.diskB)}{/snippet}
{#snippet gpu(r: Row)}{r.gpuS >= 1 ? dur(r.gpuS) : "—"}{/snippet}
{#snippet mem(r: Row)}{sizeStr(r.memPeak)}{/snippet}
{#snippet last(r: Row)}<span class="soft">{ago(r.last)}</span>{/snippet}

<div class="pg">
  <div class="pg-bar">
    <div class="pg-seg" role="radiogroup" aria-label={t("history.period")} use:pill>
      {#each RANGES as d}
        <button role="radio" aria-checked={days === d} class:on={days === d} onclick={() => (days = d)}>{d === 1 ? t("history.today") : t("history.days", { count: d })}</button>
      {/each}
    </div>
    <div class="pg-actions">
      <label class="only">
        <Switch label={t("history.onlyApps")} checked={onlyApps} onchange={(v) => (onlyApps = v)} />
        {t("history.onlyApps")}
      </label>
      <button class="btn" onclick={() => (confirm = true)} disabled={!rows.length}>{t("history.clear")}</button>
    </div>
  </div>
  <div class="pg-panel">
    <DataTable
      label={t("nav.history")}
      rows={shown}
      rowKey={(r) => r.app}
      bind:selected
      bind:sortKey
      bind:sortDir
      empty={loaded ? t("history.empty") : t("history.loading")}
      cols={[
        { key: "name", label: t("common.name"), width: "minmax(220px, 1.4fr)", sort: (r) => r.app.toLowerCase(), cell: name },
        { key: "cpu", label: t("history.cpuTime"), width: "minmax(170px, 1fr)", align: "right", sort: (r) => r.cpuS, cell: cpu },
        { key: "disk", label: t("history.diskIo"), width: "150px", align: "right", sort: (r) => r.diskB, cell: disk },
        { key: "gpu", label: t("history.gpuTime"), width: "110px", align: "right", sort: (r) => r.gpuS, cell: gpu },
        { key: "mem", label: t("history.peakMemory"), width: "136px", align: "right", sort: (r) => r.memPeak, cell: mem },
        { key: "last", label: t("history.lastActive"), width: "150px", sort: (r) => r.last, cell: last },
      ] satisfies Col<Row>[]}
    />
  </div>
</div>

{#if confirm}
  <Dialog title={t("history.confirmTitle")} onclose={() => (confirm = false)} width={440}>
    <p class="msg">{t("history.confirmText")}</p>
    {#snippet footer()}
      <button class="btn" data-autofocus onclick={() => (confirm = false)}>{t("common.cancel")}</button>
      <button class="btn primary" onclick={clear}>{t("history.clear")}</button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .only {
    display: flex;
    align-items: center;
    gap: 9px;
    color: var(--text-2);
    margin-right: 8px;
  }
  .share {
    flex: 1;
    max-width: 80px;
    height: 3px;
    margin-right: 10px;
    border-radius: 2px;
    background: var(--line-soft);
    overflow: hidden;
  }
  .share i {
    display: block;
    height: 100%;
    background: var(--cpu);
    transform-origin: left;
  }
  .dur {
    min-width: 76px;
    text-align: right;
  }
  .msg {
    margin: 0;
    color: var(--text-2);
    line-height: 1.5;
  }
</style>

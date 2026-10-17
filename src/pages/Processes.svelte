<script lang="ts">
  import { app } from "../lib/state.svelte";
  import { act } from "../lib/api";
  import type { Section } from "../lib/types";
  import LiveStrip from "../components/LiveStrip.svelte";
  import Diagnosis from "../components/Diagnosis.svelte";
  import ProcessTable from "../components/ProcessTable.svelte";
  import Icon from "../components/Icon.svelte";
  import { pill } from "../lib/pill";
  import { t } from "../lib/i18n.svelte";

  let filter = $state<Section | "all">("all");
  const groups = $derived(app.snap?.groups ?? []);
  const count = (s: Section) => groups.filter((g) => g.section === s).length;
  // labels: processes.filters.<filter>
  const FILTERS: (Section | "all")[] = ["all", "app", "ctr", "bg", "sys"];
  const sel = $derived(app.selected);

  function showGroup(key: string) {
    filter = "all";
    app.search = "";
    app.selected = key;
    queueMicrotask(() => document.querySelector(".row.selected")?.scrollIntoView({ block: "center", behavior: "smooth" }));
  }
</script>

<div class="page">
  <LiveStrip onopen={() => (app.page = "performance")} />
  <Diagnosis onshow={showGroup} />
  <div class="bar">
    <div class="seg" role="tablist" aria-label={t("processes.filterLabel")} use:pill>
      {#each FILTERS as k}
        <button role="tab" aria-selected={filter === k} class:on={filter === k} onclick={() => (filter = k)}>
          {t(`processes.filters.${k}`)}
          {#if k !== "all"}<span>{count(k)}</span>{/if}
        </button>
      {/each}
    </div>
    <div class="acts">
      <button class="btn" onclick={() => act("run-dialog")}><Icon name="plus" size={14} />{t("processes.newTask")}</button>
      <button class="btn" disabled={!sel} onclick={() => sel && act("terminate", { key: sel })}><Icon name="x" size={14} />{t("menu.endTask")}</button>
      <button class="btn" disabled={!sel} onclick={() => sel && act("efficiency", { key: sel, on: true })}><Icon name="leaf" size={14} />{t("processes.efficiency")}</button>
    </div>
  </div>
  <div class="tablewrap">
    <ProcessTable {filter} />
  </div>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100%;
    min-height: 0;
    padding: 0 20px 18px;
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 8px 12px;
    margin-top: 2px;
  }
  .seg {
    position: relative;
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: var(--r-sm);
    background: var(--panel);
    box-shadow: var(--sheen);
    border: 1px solid var(--line-soft);
  }
  .seg button {
    white-space: nowrap;
    display: inline-flex;
    align-items: baseline;
    gap: 6px;
    height: 26px;
    padding: 0 11px;
    border-radius: 4px;
    color: var(--text-2);
    line-height: 26px;
    position: relative;
    z-index: 1;
    transition: color var(--d-small);
  }
  .seg button span {
    color: var(--faint);
    font-size: var(--fs-sm);
  }
  .seg button:hover {
    color: var(--text);
  }
  .seg button.on {
    color: var(--text);
  }
  .seg > :global(.pill) {
    border-radius: 4px;
    background: var(--raised);
    box-shadow: 0 0 0 1px var(--line);
  }
  /* on narrow windows (or with longer labels) the buttons go to a second row, still on the right */
  .acts {
    display: flex;
    gap: 8px;
    margin-left: auto;
  }
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 32px;
    padding: 0 13px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line);
    background: var(--panel);
    box-shadow: var(--sheen);
    color: var(--text);
    white-space: nowrap;
    transition: border-color 120ms, opacity 120ms;
  }
  .btn:hover:not(:disabled) {
    border-color: var(--faint);
  }
  .btn:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .tablewrap {
    flex: 1;
    min-height: 0;
    background: var(--panel-bg);
    box-shadow: var(--panel-shadow);
    border: 1px solid var(--panel-edge);
    border-radius: var(--r-md);
    overflow: hidden;
  }
</style>

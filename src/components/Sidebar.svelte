<script lang="ts">
  import { updates } from "../lib/update.svelte";
  import { app, type Page } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";
  import { pill } from "../lib/pill";
  import { t } from "../lib/i18n.svelte";

  // page and icon (the label is nav.<page> in the dictionary)
  const NAV: [Page, string][] = [
    ["processes", "processes"],
    ["performance", "performance"],
    ["history", "history"],
    ["startup", "startup"],
    ["network", "network"],
    ["users", "users"],
    ["details", "details"],
    ["services", "services"],
  ];
  const alerts = $derived(app.snap?.insights.filter((i) => i.level === "critical").length ?? 0);
</script>

<nav class="side" aria-label={t("nav.label")} use:pill={{ selector: "button.active", axis: "y" }}>
  <div class="brand" data-tauri-drag-region>
    <img src="/favicon.svg" alt="" data-tauri-drag-region />
    <span data-tauri-drag-region>{t("app.name")}</span>
  </div>
  <ul>
    {#each NAV as [key, icon]}
      <li>
        <button class:active={app.page === key} aria-current={app.page === key ? "page" : undefined} onclick={() => (app.page = key)}>
          <Icon name={icon} size={16} />
          <span>{t(`nav.${key}`)}</span>
          {#if key === "processes" && alerts}<i class="dot" title={t("nav.alerts", { count: alerts })}></i>{/if}
        </button>
      </li>
    {/each}
  </ul>
  <div class="foot">
    <button class:active={app.page === "settings"} onclick={() => (app.page = "settings")}>
      <Icon name="settings" size={16} />
      <span>{t("nav.settings")}</span>
      {#if updates.available}<i class="dot upd" title={t("settings.updates.dot")}></i>{/if}
    </button>
  </div>
</nav>

<style>
  .side {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 0 10px 12px;
    border-right: 1px solid var(--line-soft);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 56px;
    padding: 0 8px;
    font-weight: 600;
    color: var(--text);
  }
  .brand img {
    width: 22px;
    height: 22px;
  }
  ul {
    list-style: none;
    margin: 4px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  button {
    position: relative;
    display: flex;
    align-items: center;
    gap: 11px;
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border-radius: var(--r-sm);
    color: var(--text-2);
    text-align: left;
    z-index: 1;
    transition:
      background-color var(--d-micro),
      color var(--d-small);
  }
  button :global(svg) {
    transition:
      color var(--d-small),
      transform var(--d-small) var(--ease-out);
  }
  button:active :global(svg) {
    transform: scale(0.9);
  }
  button :global(svg) {
    color: var(--muted);
    flex: none;
  }
  button:hover {
    background: var(--hover);
    color: var(--text);
  }
  button.active {
    color: var(--text);
    font-weight: 500;
  }
  button.active:hover {
    background: transparent;
  }
  /* active item indicator: slides from one item to the next */
  .side > :global(.pill) {
    border-radius: var(--r-sm);
    background: var(--select);
    box-shadow: var(--sheen);
  }
  .side > :global(.pill)::before {
    content: "";
    position: absolute;
    left: 0;
    top: 9px;
    bottom: 9px;
    width: 2px;
    border-radius: 2px;
    background: var(--text);
  }
  button.active :global(svg) {
    color: var(--text);
  }
  .dot {
    margin-left: auto;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--crit);
    animation: dot-in var(--d-large) var(--ease-spring) both;
  }
  @keyframes dot-in {
    from {
      transform: scale(0);
    }
  }
  .dot.upd {
    background: var(--cpu);
  }
  .foot {
    margin-top: auto;
  }
</style>

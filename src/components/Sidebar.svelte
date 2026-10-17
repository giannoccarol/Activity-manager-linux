<script lang="ts">
  import { app, type Page } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";
  import { pill } from "../lib/pill";

  const NAV: [Page, string, string][] = [
    ["processes", "Processi", "processes"],
    ["performance", "Prestazioni", "performance"],
    ["history", "Cronologia app", "history"],
    ["startup", "App di avvio", "startup"],
    ["network", "Rete", "network"],
    ["users", "Utenti", "users"],
    ["details", "Dettagli", "details"],
    ["services", "Servizi", "services"],
  ];
  const alerts = $derived(app.snap?.insights.filter((i) => i.level === "critical").length ?? 0);
</script>

<nav class="side" aria-label="Sezioni" use:pill={{ selector: "button.active", axis: "y" }}>
  <div class="brand" data-tauri-drag-region>
    <img src="/favicon.svg" alt="" data-tauri-drag-region />
    <span data-tauri-drag-region>Gestione attività</span>
  </div>
  <ul>
    {#each NAV as [key, label, icon]}
      <li>
        <button class:active={app.page === key} aria-current={app.page === key ? "page" : undefined} onclick={() => (app.page = key)}>
          <Icon name={icon} size={16} />
          <span>{label}</span>
          {#if key === "processes" && alerts}<i class="dot" title="{alerts} problemi da guardare"></i>{/if}
        </button>
      </li>
    {/each}
  </ul>
  <div class="foot">
    <button class:active={app.page === "settings"} onclick={() => (app.page = "settings")}>
      <Icon name="settings" size={16} />
      <span>Impostazioni</span>
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
  /* indicatore della voce attiva: scorre da una voce all'altra */
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
  .foot {
    margin-top: auto;
  }
</style>

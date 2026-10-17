<script lang="ts">
  import { invoke, act } from "../lib/api";
  import { ui } from "../lib/ui.svelte";
  import { app } from "../lib/state.svelte";
  import { mockStartup } from "../lib/mockpages";
  import type { Col } from "../lib/table";
  import type { StartupItem } from "../lib/types";
  import DataTable from "../components/DataTable.svelte";
  import AppIcon from "../components/AppIcon.svelte";
  import Switch from "../components/Switch.svelte";
  import ContextMenu, { type MenuItem } from "../components/ContextMenu.svelte";

  // Apps that start at login (XDG autostart folders), can be enabled and disabled as in KDE.
  let items = $state<StartupItem[]>([]);
  let loaded = $state(false);
  let selected = $state<string | null>(null);
  let sortKey = $state("name");
  let sortDir = $state<1 | -1>(1);
  let busy = $state<string | null>(null);
  let menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);

  async function load() {
    try {
      items = await invoke<StartupItem[]>("startup_list", {}, mockStartup);
    } catch (e) {
      ui.toast(String(e), "crit");
    }
    loaded = true;
  }
  $effect(() => {
    load();
  });

  async function toggle(it: StartupItem, on: boolean) {
    busy = it.id;
    try {
      await invoke("startup_set", { id: it.id, enabled: on }, () => null);
      if (!("__TAURI_INTERNALS__" in window)) it.enabled = on;
      ui.toast(on ? `${it.name} partirà all'accesso` : `${it.name} non partirà più all'accesso`);
      await load();
    } catch (e) {
      ui.toast(String(e), "crit");
    }
    busy = null;
  }

  const needle = $derived(app.search.trim().toLowerCase());
  const shown = $derived(items.filter((i) => !needle || i.name.toLowerCase().includes(needle) || i.exec.toLowerCase().includes(needle)));
  const on = $derived(items.filter((i) => i.enabled).length);
  const sel = $derived(items.find((i) => i.id === selected) ?? null);
</script>

{#snippet name(it: StartupItem)}
  <AppIcon src={it.icon} section="app" size={20} />
  <span class="stack">
    <span class="t">{it.name}</span>
    {#if it.comment}<span class="faint t sub">{it.comment}</span>{/if}
  </span>
{/snippet}
{#snippet source(it: StartupItem)}<span class="soft">{it.source === "user" ? "Aggiunta da te" : "Di sistema"}</span>{/snippet}
{#snippet exec(it: StartupItem)}<span class="faint t" title={it.exec}>{it.exec}</span>{/snippet}
{#snippet onoff(it: StartupItem)}
  <Switch label="Avvia {it.name} all'accesso" checked={it.enabled} disabled={busy === it.id} onchange={(v) => toggle(it, v)} />
  <span class="soft st">{it.enabled ? "Attiva" : "Disattivata"}</span>
{/snippet}

<div class="pg">
  <div class="pg-bar">
    <span class="note">{on} app partono all'accesso{items.length - on ? `, ${items.length - on} disattivate` : ""}</span>
    <div class="pg-actions">
      <button class="btn" disabled={!sel} onclick={() => sel && toggle(sel, !sel.enabled)}>{sel?.enabled === false ? "Attiva" : "Disattiva"}</button>
    </div>
  </div>
  <div class="pg-panel">
    <DataTable
      label="App di avvio"
      rows={shown}
      rowKey={(i) => i.id}
      bind:selected
      bind:sortKey
      bind:sortDir
      empty={loaded ? "Nessuna app parte all'accesso" : "Carico l'elenco…"}
      rowHeight={48}
      onmenu={(e, it) =>
        (menu = {
          x: e.clientX,
          y: e.clientY,
          items: [
            { label: it.enabled ? "Disattiva" : "Attiva", run: () => toggle(it, !it.enabled) },
            { sep: true },
            { label: "Apri il file di avvio", icon: "folder", run: () => act("run", { cmd: `xdg-open '${it.path.replace(/'/g, "'\\''")}'` }) },
            { label: "Copia il comando", icon: "copy", run: () => { navigator.clipboard?.writeText(it.exec); ui.toast("Comando copiato"); } },
          ],
        })}
      cols={[
        { key: "name", label: "Nome", width: "minmax(260px, 1.4fr)", sort: (i) => i.name.toLowerCase(), cell: name },
        { key: "source", label: "Origine", width: "140px", sort: (i) => i.source, cell: source },
        { key: "exec", label: "Comando", width: "minmax(180px, 1fr)", sort: (i) => i.exec, cell: exec },
        { key: "state", label: "All'accesso", width: "150px", sort: (i) => (i.enabled ? 1 : 0), cell: onoff },
      ] satisfies Col<StartupItem>[]}
    />
  </div>
</div>
{#if menu}<ContextMenu {...menu} onclose={() => (menu = null)} />{/if}

<style>
  .stack {
    display: flex;
    flex-direction: column;
    min-width: 0;
    padding: 6px 0;
    line-height: 1.3;
  }
  .sub {
    font-size: var(--fs-sm);
  }
  .st {
    min-width: 76px;
  }
</style>

<script lang="ts">
  import { rise } from "../lib/motion.svelte";
  import { app } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";
  import WindowControls from "./WindowControls.svelte";

  let { title, searchable = true }: { title: string; searchable?: boolean } = $props();
  let input: HTMLInputElement | undefined = $state();

  function key(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f" && searchable) {
      e.preventDefault();
      input?.focus();
      input?.select();
    }
  }
</script>

<svelte:window onkeydown={key} />
<header class="top" data-tauri-drag-region>
  {#key title}<h1 data-tauri-drag-region in:rise={{ y: 4, duration: 220 }}>{title}</h1>{/key}
  <div class="right">
    {#if searchable}
      <label class="search">
        <Icon name="search" size={15} />
        <input bind:this={input} bind:value={app.search} placeholder="Cerca un'app, un PID o un comando" spellcheck="false"
          onkeydown={(e) => e.key === "Escape" && (app.search = "")} />
        {#if app.search}<button aria-label="Cancella ricerca" onclick={() => (app.search = "")}><Icon name="x" size={13} /></button>{/if}
      </label>
    {/if}
    <WindowControls />
  </div>
</header>

<style>
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    height: 56px;
    padding: 0 12px 0 24px;
  }
  h1 {
    margin: 0;
    font-size: 24px;
    font-weight: 650;
    letter-spacing: -0.01em;
  }
  .right {
    display: flex;
    align-items: center;
    gap: 18px;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 340px;
    height: 32px;
    padding: 0 10px;
    border-radius: var(--r-sm);
    background: var(--panel);
    box-shadow: var(--sheen);
    border: 1px solid var(--line);
    color: var(--muted);
    transition: border-color 120ms;
  }
  .search:focus-within {
    border-color: var(--faint);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
  }
  .search input::placeholder {
    color: var(--faint);
  }
  .search button {
    display: grid;
    color: var(--muted);
  }
</style>

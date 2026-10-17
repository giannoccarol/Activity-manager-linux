<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";
  import { fadeq, pop } from "../lib/motion.svelte";

  let { title, onclose, children, footer, width = 520 }: { title: string; onclose: () => void; children: Snippet; footer?: Snippet; width?: number } = $props();
  const id = $props.id();
  let box: HTMLDivElement;

  $effect(() => {
    const prev = document.activeElement as HTMLElement | null;
    (box.querySelector("input, textarea, [data-autofocus]") as HTMLElement | null)?.focus();
    return () => prev?.focus?.();
  });

  function key(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      onclose();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
<div class="scrim" onclick={onclose} onkeydown={key} transition:fadeq={{ duration: 160 }}>
  <div bind:this={box} transition:pop={{ scale: 0.97, duration: 200 }} class="dialog" role="dialog" tabindex="-1" aria-modal="true" aria-labelledby={id} style:width="{width}px" onclick={(e) => e.stopPropagation()}>
    <header>
      <h2 {id}>{title}</h2>
      <button class="x" aria-label="Chiudi" onclick={onclose}><Icon name="x" size={15} /></button>
    </header>
    <div class="body">{@render children()}</div>
    {#if footer}<footer>{@render footer()}</footer>{/if}
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    background: rgb(0 0 0 / 0.32);
  }
  .dialog {
    max-width: calc(100vw - 48px);
    max-height: calc(100vh - 64px);
    display: flex;
    flex-direction: column;
    background: var(--raised);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    box-shadow: var(--shadow);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 16px 4px 20px;
  }
  h2 {
    margin: 0;
    font-size: var(--fs-lg);
    font-weight: 600;
  }
  .x {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: var(--r-sm);
    color: var(--muted);
  }
  .x:hover {
    background: var(--hover);
    color: var(--text);
  }
  .body {
    padding: 8px 20px 18px;
    overflow: auto;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--line-soft);
  }
</style>

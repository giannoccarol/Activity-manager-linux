<script module lang="ts">
  export type MenuItem =
    | { sep: true }
    | { sep?: false; label: string; icon?: string; danger?: boolean; checked?: boolean; disabled?: boolean; run: () => void };
</script>

<script lang="ts">
  import Icon from "./Icon.svelte";
  import { fadeq, pop } from "../lib/motion.svelte";

  let { x, y, items, onclose }: { x: number; y: number; items: MenuItem[]; onclose: () => void } = $props();
  let el: HTMLDivElement | undefined = $state();
  let pos = $state({ left: x, top: y });

  $effect(() => {
    if (!el) return;
    const r = el.getBoundingClientRect();
    pos = {
      left: Math.min(x, window.innerWidth - r.width - 8),
      top: y + r.height > window.innerHeight - 8 ? Math.max(8, y - r.height) : y,
    };
    el.querySelector<HTMLButtonElement>("button:not([disabled])")?.focus();
  });

  function pick(it: MenuItem) {
    if ("run" in it && !it.disabled) {
      it.run();
      onclose();
    }
  }
  function key(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const btns = [...(el?.querySelectorAll<HTMLButtonElement>("button:not([disabled])") ?? [])];
      const i = btns.indexOf(document.activeElement as HTMLButtonElement);
      btns[(i + (e.key === "ArrowDown" ? 1 : -1) + btns.length) % btns.length]?.focus();
    }
  }
</script>

<svelte:window onkeydown={key} />
<div class="scrim" role="presentation" onpointerdown={onclose} oncontextmenu={(e) => { e.preventDefault(); onclose(); }}></div>
<div class="menu" role="menu" bind:this={el} style="left:{pos.left}px; top:{pos.top}px" in:pop={{ scale: 0.95, duration: 160 }} out:fadeq={{ duration: 100 }}>
  {#each items as it, i}
    {#if it.sep}
      <hr />
    {:else}
      <button role="menuitem" class:danger={it.danger} disabled={it.disabled} onclick={() => pick(it)} style:--i={i}>
        <span class="ic">{#if it.checked}<Icon name="leaf" size={14} />{:else if it.icon}<Icon name={it.icon} size={14} />{/if}</span>
        <span>{it.label}</span>
        {#if it.checked}<span class="on">attiva</span>{/if}
      </button>
    {/if}
  {/each}
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
  }
  .menu {
    position: fixed;
    z-index: 41;
    min-width: 220px;
    padding: 5px;
    border-radius: var(--r-md);
    background: var(--raised);
    border: 1px solid var(--line);
    box-shadow: var(--shadow);
    transform-origin: top left;
  }
  button {
    animation: item-in var(--d-small) var(--ease-out) both;
    animation-delay: calc(var(--i, 0) * 12ms);
    display: grid;
    grid-template-columns: 18px 1fr auto;
    gap: 8px;
    align-items: center;
    width: 100%;
    height: 32px;
    padding: 0 10px;
    border-radius: var(--r-sm);
    text-align: left;
    color: var(--text);
  }
  button:hover,
  button:focus-visible {
    background: var(--hover);
    outline: none;
  }
  button.danger {
    color: var(--crit);
  }
  button:disabled {
    color: var(--faint);
    cursor: default;
  }
  .ic {
    color: var(--muted);
    display: grid;
  }
  .on {
    color: var(--muted);
    font-size: var(--fs-xs);
  }
  @keyframes item-in {
    from {
      opacity: 0;
      transform: translateY(-3px);
    }
  }
  hr {
    border: 0;
    height: 1px;
    background: var(--line-soft);
    margin: 4px 6px;
  }
</style>

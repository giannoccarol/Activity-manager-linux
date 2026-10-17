<script lang="ts">
  import { ui } from "../lib/ui.svelte";
  import { flip } from "svelte/animate";
  import { easeOut, fadeq, flipMs, rise } from "../lib/motion.svelte";
</script>

<div class="stack" aria-live="polite">
  {#each ui.toasts as t (t.id)}
    <div class="toast" class:crit={t.level === "crit"} in:rise={{ y: 14, duration: 260 }} out:fadeq={{ duration: 180 }} animate:flip={{ duration: flipMs(), easing: easeOut }}>{t.text}</div>
  {/each}
</div>

<style>
  .stack {
    position: fixed;
    left: 50%;
    bottom: 22px;
    z-index: 70;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    transform: translateX(-50%);
    pointer-events: none;
  }
  .toast {
    max-width: 560px;
    padding: 9px 16px;
    border-radius: var(--r-md);
    background: var(--text);
    color: var(--on-text);
    box-shadow: var(--shadow);
    line-height: 1.4;
  }
  .crit {
    background: var(--raised);
    color: var(--text);
    border: 1px solid var(--line);
    border-left: 3px solid var(--crit);
  }
</style>

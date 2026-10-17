<script lang="ts">
  let { checked, label, onchange, disabled = false }: { checked: boolean; label: string; onchange: (v: boolean) => void; disabled?: boolean } = $props();
</script>

<button class="sw" role="switch" aria-checked={checked} aria-label={label} {disabled} class:on={checked} onclick={() => onchange(!checked)}>
  <i></i>
</button>

<style>
  .sw {
    position: relative;
    flex: none;
    width: 38px;
    height: 22px;
    border-radius: 11px;
    background: var(--line);
    transition: background-color var(--d-medium) var(--ease-out);
  }
  .sw.on {
    background: var(--text);
  }
  .sw:disabled {
    opacity: 0.45;
  }
  i {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text);
    transition:
      transform var(--d-medium) var(--ease-spring),
      background-color var(--d-medium);
  }
  .on i {
    transform: translateX(16px);
    background: var(--on-text);
  }
</style>

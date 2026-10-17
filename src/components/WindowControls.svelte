<script lang="ts">
  // Pulsanti finestra in stile Breeze, a destra come in KDE.
  import { windowCmd } from "../lib/api";
  import Icon from "./Icon.svelte";
</script>

<div class="wc">
  <button aria-label="Riduci a icona" title="Riduci a icona" onclick={() => windowCmd("minimize")}><Icon name="chev-down" size={14} stroke={1.8} /></button>
  <button aria-label="Ingrandisci" title="Ingrandisci" onclick={() => windowCmd("toggleMaximize")}><Icon name="chev-up" size={14} stroke={1.8} /></button>
  <button class="close" aria-label="Chiudi" title="Chiudi" onclick={() => windowCmd("close")}><Icon name="x" size={14} stroke={1.8} /></button>
</div>

<style>
  .wc {
    display: flex;
    gap: 6px;
  }
  button {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    color: var(--text-2);
    transition: background-color 120ms, color 120ms;
  }
  button:hover {
    background: var(--select);
    color: var(--text);
  }
  button.close:hover {
    background: var(--crit);
    color: #fff;
  }
</style>

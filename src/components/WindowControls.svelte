<script lang="ts">
  // Breeze-style window buttons, on the right as in KDE.
  import { windowCmd } from "../lib/api";
  import Icon from "./Icon.svelte";
  import { t } from "../lib/i18n.svelte";
</script>

<div class="wc">
  <button aria-label={t("topbar.minimize")} title={t("topbar.minimize")} onclick={() => windowCmd("minimize")}><Icon name="chev-down" size={14} stroke={1.8} /></button>
  <button aria-label={t("topbar.maximize")} title={t("topbar.maximize")} onclick={() => windowCmd("toggleMaximize")}><Icon name="chev-up" size={14} stroke={1.8} /></button>
  <button class="close" aria-label={t("common.close")} title={t("common.close")} onclick={() => windowCmd("close")}><Icon name="x" size={14} stroke={1.8} /></button>
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

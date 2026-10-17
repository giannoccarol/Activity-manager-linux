<script lang="ts">
  import { settings, type Theme } from "../lib/settings.svelte";
  import Switch from "../components/Switch.svelte";
  import { pill } from "../lib/pill";
  import { themeFade } from "../lib/motion.svelte";
  import { t, type LangPref } from "../lib/i18n.svelte";
  import { num } from "../lib/format";

  // name and description: settings.themes.<theme>.name / .desc
  const THEMES: Theme[] = ["graphite", "paper", "glass"];
  const LANGS: [LangPref, string][] = [
    ["auto", "settings.language.auto"],
    ["it", "settings.language.it"],
    ["en", "settings.language.en"],
  ];
  const RATES = [500, 1000, 2000, 5000];
  const s = $derived(settings.v);

  // a sentence with {placeholders} split into text and placeholder names, to put key caps in their place
  const pieces = (key: string) => t(key).split(/(\{\w+\})/).filter(Boolean);
</script>

{#snippet secureKeys()}<span class="keys"><kbd>Ctrl</kbd><kbd>Alt</kbd><kbd>{t("settings.keys.del")}</kbd></span>{/snippet}
{#snippet taskKeys()}<span class="keys"><kbd>Ctrl</kbd><kbd>{t("settings.keys.shift")}</kbd><kbd>Esc</kbd></span>{/snippet}

<div class="page">
  <section>
    <h2>{t("settings.appearance.title")}</h2>
    <div class="themes" role="radiogroup" aria-label={t("settings.appearance.theme")}>
      {#each THEMES as id}
        <button class="theme" role="radio" aria-checked={s.theme === id} class:on={s.theme === id} onclick={() => themeFade(() => settings.set({ theme: id }))}>
          <span class="pv pv-{id}">
            <span class="pv-side"><i></i><i></i><i></i><i></i></span>
            <span class="pv-main">
              <span class="pv-strip"><i class="c1"></i><i class="c2"></i><i class="c3"></i></span>
              <span class="pv-table"><i></i><i></i><i></i><i></i></span>
            </span>
          </span>
          <span class="tname">{t(`settings.themes.${id}.name`)}</span>
          <span class="tdesc">{t(`settings.themes.${id}.desc`)}</span>
        </button>
      {/each}
    </div>
  </section>

  <section>
    <h2>{t("settings.language.title")}</h2>
    <div class="row">
      <div class="txt">
        <span class="label">{t("settings.language.label")}</span>
        <span class="desc">{t("settings.language.desc")}</span>
      </div>
      <div class="seg" role="radiogroup" aria-label={t("settings.language.label")} use:pill>
        {#each LANGS as [id, key]}
          <button role="radio" aria-checked={s.language === id} class:on={s.language === id} onclick={() => settings.set({ language: id })}>{t(key)}</button>
        {/each}
      </div>
    </div>
  </section>

  <section>
    <h2>{t("settings.refresh.title")}</h2>
    <div class="row">
      <div class="txt">
        <span class="label">{t("settings.refresh.label")}</span>
        <span class="desc">{t("settings.refresh.desc")}</span>
      </div>
      <div class="seg" role="radiogroup" aria-label={t("settings.refresh.label")} use:pill>
        {#each RATES as ms}
          <button role="radio" aria-checked={s.intervalMs === ms} class:on={s.intervalMs === ms} onclick={() => settings.set({ intervalMs: ms })}>{t("format.seconds", { s: num(ms / 1000, ms % 1000 ? 1 : 0) })}</button>
        {/each}
      </div>
    </div>
  </section>

  <section>
    <h2>{t("settings.keyboard.title")}</h2>
    <div class="row">
      <div class="txt">
        <span class="label">{t("settings.keyboard.label")}</span>
        <span class="desc">{#each pieces("settings.keyboard.desc") as p}{#if p === "{secure}"}{@render secureKeys()}{:else if p === "{taskmgr}"}{@render taskKeys()}{:else}{p}{/if}{/each}</span>
      </div>
      <Switch label={t("settings.keyboard.label")} checked={s.shortcuts} onchange={(v) => settings.set({ shortcuts: v })} />
    </div>
  </section>

  <section>
    <h2>{t("settings.system.title")}</h2>
    <div class="row">
      <div class="txt">
        <span class="label">{t("settings.alerts.label")}</span>
        <span class="desc">{t("settings.alerts.desc")}</span>
      </div>
      <Switch label={t("settings.alerts.switch")} checked={s.alerts} onchange={(v) => settings.set({ alerts: v })} />
    </div>
    <div class="row">
      <div class="txt">
        <span class="label">{t("settings.tray.label")}</span>
        <span class="desc">{t("settings.tray.desc")}</span>
      </div>
      <Switch label={t("settings.tray.label")} checked={s.tray} onchange={(v) => settings.set({ tray: v })} />
    </div>
    <div class="row">
      <div class="txt">
        <span class="label">{t("settings.autostart.label")}</span>
        <span class="desc">{#each pieces("settings.autostart.desc") as p}{#if p === "{keys}"}{@render secureKeys()}{:else}{p}{/if}{/each}</span>
      </div>
      <Switch label={t("settings.autostart.label")} checked={s.autostart} onchange={(v) => settings.set({ autostart: v })} />
    </div>
  </section>

  <p class="about">{t("settings.about")}</p>
</div>

<style>
  .page {
    height: 100%;
    overflow-y: auto;
    padding: 4px 28px 32px 20px;
  }
  section {
    max-width: 820px;
    padding: 18px 0 8px;
    border-bottom: 1px solid var(--line-soft);
  }
  section:last-of-type {
    border-bottom: 0;
  }
  h2 {
    margin: 0 0 12px;
    font-size: var(--fs-md);
    font-weight: 600;
  }
  .themes {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 200px));
    gap: 16px;
    margin-bottom: 10px;
  }
  .theme {
    display: flex;
    flex-direction: column;
    gap: 2px;
    text-align: left;
    padding: 0;
  }
  .pv {
    position: relative;
    display: flex;
    aspect-ratio: 16 / 10;
    margin-bottom: 8px;
    overflow: hidden;
    border-radius: var(--r-md);
    border: 1px solid var(--line);
    background-size: cover;
    background-position: center;
    transition:
      box-shadow var(--d-small) var(--ease-out),
      transform var(--d-medium) var(--ease-out);
  }
  .theme:hover .pv {
    box-shadow: 0 0 0 1px var(--faint);
    transform: translateY(-2px);
  }
  .theme:active .pv {
    transform: translateY(0) scale(0.98);
  }
  .theme.on .pv {
    box-shadow: 0 0 0 2px var(--text);
  }
  .tname {
    font-weight: 600;
  }
  .tdesc {
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  /* previews: each theme with its own colors, independent of the active one */
  .pv-side {
    display: flex;
    flex-direction: column;
    gap: 5px;
    width: 26%;
    padding: 10px 8px;
  }
  .pv-side i {
    height: 5px;
    border-radius: 3px;
    background: var(--pv-line);
  }
  .pv-side i:first-child {
    width: 70%;
    background: var(--pv-text);
    opacity: 0.8;
    margin-bottom: 4px;
  }
  .pv-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 10px 10px 0;
  }
  .pv-strip,
  .pv-table {
    display: flex;
    gap: 4px;
    padding: 6px;
    border-radius: 5px;
    background: var(--pv-panel);
    border: 1px solid var(--pv-border);
  }
  .pv-strip i {
    flex: 1;
    height: 14px;
    border-radius: 2px;
    border-bottom: 2px solid;
  }
  .c1 {
    border-color: #5aa2f5 !important;
  }
  .c2 {
    border-color: #2dd4bf !important;
  }
  .c3 {
    border-color: #4ade80 !important;
  }
  .pv-table {
    flex: 1;
    flex-direction: column;
  }
  .pv-table i {
    height: 4px;
    border-radius: 2px;
    background: var(--pv-line);
  }
  .pv-graphite {
    --pv-panel: #202328;
    --pv-border: #30353b;
    --pv-line: #3a4047;
    --pv-text: #e9ebee;
    background-color: #1a1d21;
  }
  .pv-paper {
    --pv-panel: #f6f7f8;
    --pv-border: #d9dde2;
    --pv-line: #d3d8de;
    --pv-text: #16181b;
    background-color: #eceef1;
  }
  .pv-glass {
    --pv-panel: rgb(255 255 255 / 0.09);
    --pv-border: rgb(255 255 255 / 0.16);
    --pv-line: rgb(255 255 255 / 0.22);
    --pv-text: #fff;
    background:
      radial-gradient(circle at 20% 25%, #3d7fb0 0, transparent 45%),
      radial-gradient(circle at 80% 70%, #3a8f88 0, transparent 45%),
      #22303f;
  }
  .pv-glass::before {
    content: "";
    position: absolute;
    inset: 0;
    background: rgb(10 14 20 / 0.4);
  }
  .pv-glass > * {
    position: relative;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 12px 0;
  }
  .row + .row {
    border-top: 1px solid var(--line-soft);
  }
  .txt {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .label {
    font-weight: 500;
  }
  .desc {
    color: var(--muted);
    line-height: 1.5;
    max-width: 62ch;
  }
  .ctl {
    display: flex;
    gap: 8px;
    flex: none;
  }
  .keys {
    display: inline-flex;
    gap: 3px;
    vertical-align: 1px;
  }
  kbd {
    font: inherit;
    font-size: var(--fs-xs);
    padding: 1px 6px;
    border-radius: 4px;
    border: 1px solid var(--line);
    border-bottom-width: 2px;
    color: var(--text-2);
    background: var(--panel);
  }
  .seg {
    position: relative;
    display: flex;
    flex: none;
    padding: 3px;
    border-radius: var(--r-sm);
    background: var(--bg);
    border: 1px solid var(--line-soft);
  }
  .seg button {
    position: relative;
    z-index: 1;
    height: 26px;
    padding: 0 12px;
    border-radius: 4px;
    color: var(--muted);
    transition: color var(--d-small);
  }
  .seg button.on {
    color: var(--text);
  }
  .seg > :global(.pill) {
    border-radius: 4px;
    background: var(--raised);
    box-shadow: var(--sheen);
  }
  .about {
    max-width: 820px;
    margin: 18px 0 0;
    color: var(--faint);
    font-size: var(--fs-sm);
  }
</style>

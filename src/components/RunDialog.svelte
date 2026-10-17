<script lang="ts">
  import Dialog from "./Dialog.svelte";
  import { act } from "../lib/api";
  import { ui } from "../lib/ui.svelte";
  import { t } from "../lib/i18n.svelte";

  const KEY = "ga.run-history";
  const recent: string[] = (() => {
    try {
      return JSON.parse(localStorage.getItem(KEY) ?? "[]");
    } catch {
      return [];
    }
  })();
  let cmd = $state(recent[0] ?? "");
  let admin = $state(false);
  let busy = $state(false);

  const close = () => (ui.dialog = null);

  async function run(e: SubmitEvent) {
    e.preventDefault();
    const c = cmd.trim();
    if (!c || busy) return;
    busy = true;
    const r = await act("run", { cmd: c, admin });
    busy = false;
    if (r === null && "__TAURI_INTERNALS__" in window) return;
    try {
      localStorage.setItem(KEY, JSON.stringify([c, ...recent.filter((x) => x !== c)].slice(0, 12)));
    } catch {}
    ui.toast(t("toasts.started", { cmd: c.split(" ")[0] }));
    close();
  }
</script>

<Dialog title={t("dialogs.run.title")} onclose={close} width={480}>
  <form id="run" onsubmit={run}>
    <p>{t("dialogs.run.intro")}</p>
    <input bind:value={cmd} list="run-recent" spellcheck="false" autocomplete="off" placeholder={t("dialogs.run.placeholder")} onfocus={(e) => e.currentTarget.select()} />
    <datalist id="run-recent">
      {#each recent as r}<option value={r}></option>{/each}
    </datalist>
    <label>
      <input type="checkbox" bind:checked={admin} />
      {t("dialogs.run.admin")}
    </label>
  </form>
  {#snippet footer()}
    <button class="btn" onclick={close}>{t("common.cancel")}</button>
    <button class="btn primary" type="submit" form="run" disabled={!cmd.trim() || busy}>{t("dialogs.run.run")}</button>
  {/snippet}
</Dialog>

<style>
  p {
    margin: 0 0 14px;
    color: var(--text-2);
    line-height: 1.5;
  }
  input:not([type]) {
    width: 100%;
    height: 38px;
    padding: 0 12px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: var(--fs-md);
  }
  input:not([type]):focus {
    outline: none;
    border-color: var(--muted);
  }
  label {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 14px;
    color: var(--text-2);
  }
  input[type="checkbox"] {
    accent-color: var(--text);
    width: 15px;
    height: 15px;
    margin: 0;
  }
</style>

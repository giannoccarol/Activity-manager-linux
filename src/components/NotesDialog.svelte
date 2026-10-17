<script lang="ts">
  import Dialog from "./Dialog.svelte";
  import { ui } from "../lib/ui.svelte";
  import { notesHtml, updates } from "../lib/update.svelte";
  import { invoke } from "../lib/api";
  import { i18n, t } from "../lib/i18n.svelte";

  // Release notes of the new version, with the button to update.
  const close = () => (ui.dialog = null);
  const info = $derived(updates.info);
  const date = $derived(info?.published ? new Date(info.published).toLocaleDateString(i18n.tag, { day: "numeric", month: "long", year: "numeric" }) : "");
  const html = $derived(notesHtml(info?.notes ?? ""));
</script>

<Dialog title={t("dialogs.notes.title", { version: info?.latest ?? "" })} onclose={close} width={560}>
  {#if date}<p class="date">{t("dialogs.notes.published", { date })}</p>{/if}
  <div class="notes">{@html html}</div>
  {#snippet footer()}
    {#if info?.url}<button class="btn" onclick={() => invoke("open_url", { url: info.url }, () => null)}>{t("dialogs.notes.github")}</button>{/if}
    <span class="grow"></span>
    <button class="btn" data-autofocus onclick={close}>{t("common.close")}</button>
    {#if info?.canInstall && updates.available}
      <button
        class="btn primary"
        onclick={() => {
          close();
          updates.install();
        }}>{t("settings.updates.install")}</button>
    {/if}
  {/snippet}
</Dialog>

<style>
  .date {
    margin: 0 0 10px;
    color: var(--muted);
  }
  .notes {
    max-height: 52vh;
    overflow: auto;
    color: var(--text-2);
    line-height: 1.55;
    user-select: text;
  }
  .notes :global(h4) {
    margin: 14px 0 6px;
    color: var(--text);
    font-size: var(--fs-md);
    font-weight: 600;
  }
  .notes :global(h4:first-child) {
    margin-top: 0;
  }
  .notes :global(p) {
    margin: 0 0 8px;
  }
  .notes :global(ul) {
    margin: 0 0 8px;
    padding-left: 18px;
  }
  .notes :global(li) {
    margin: 3px 0;
  }
  .notes :global(code) {
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--bg);
    font-size: var(--fs-sm);
  }
  .notes :global(pre) {
    margin: 0 0 8px;
    padding: 8px 10px;
    border-radius: var(--r-sm);
    background: var(--bg);
    overflow: auto;
    font-size: var(--fs-sm);
  }
  .notes :global(strong) {
    color: var(--text);
  }
  .notes :global(hr) {
    border: 0;
    height: 1px;
    background: var(--line-soft);
    margin: 10px 0;
  }
  .grow {
    flex: 1;
  }
</style>

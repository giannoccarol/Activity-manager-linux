<script lang="ts">
  import { i18n, t } from "../lib/i18n.svelte";
  import Dialog from "./Dialog.svelte";
  import Icon from "./Icon.svelte";
  import { ui, type Props } from "../lib/ui.svelte";
  import { act } from "../lib/api";
  import { uptime } from "../lib/format";

  let { data }: { data: Props } = $props();
  const close = () => (ui.dialog = null);

  const started = $derived(new Date(data.started * 1000));
  const since = $derived(uptime(Date.now() / 1000 - data.started));
  const when = $derived(
    started.toLocaleString(i18n.tag, { weekday: "long", day: "numeric", month: "long", hour: "2-digit", minute: "2-digit", second: "2-digit" }),
  );
  const prio = $derived(data.nice == null ? "" : t(data.nice >= 19 ? "priority.lowest" : data.nice > 0 ? "priority.low" : data.nice < 0 ? "priority.high" : "priority.normal"));

  function copy(v: string) {
    navigator.clipboard?.writeText(v);
    ui.toast(t("toasts.copied"));
  }
</script>

<Dialog title={data.group && data.group !== data.name ? t("dialogs.props.inGroup", { name: data.name, group: data.group }) : data.name} onclose={close} width={600}>
  <dl>
    <dt>{t("dialogs.props.program")}</dt>
    <dd class="path">
      <span>{data.exe || t("dialogs.props.notAccessible")}</span>
      {#if data.exe}<button aria-label={t("dialogs.props.openLocation")} title={t("dialogs.props.openLocation")} onclick={() => act("open-location", { pid: data.pid })}><Icon name="folder" size={14} /></button>{/if}
    </dd>
    <dt>{t("common.commandLine")}</dt>
    <dd class="path">
      <span class="cmd">{data.cmd || "—"}</span>
      {#if data.cmd}<button aria-label={t("dialogs.props.copy")} title={t("dialogs.props.copy")} onclick={() => copy(data.cmd)}><Icon name="copy" size={14} /></button>{/if}
    </dd>
    <dt>{t("dialogs.props.cwd")}</dt>
    <dd>{data.cwd || t("dialogs.props.notAccessible")}</dd>
    <dt>{t("dialogs.props.started")}</dt>
    <dd>{t("dialogs.props.startedAgo", { when, since })}</dd>
    <dt>PID</dt>
    <dd>{data.pid}{#if data.ppid}<span class="soft">, {t("dialogs.props.childOf", { pid: data.ppid })}</span>{/if}</dd>
    <dt>{t("dialogs.props.physicalMemory")}</dt>
    <dd>{data.vmRss || "—"}{#if data.vmSwap && data.vmSwap !== "0 kB"}<span class="soft">, {t("dialogs.props.swap", { size: data.vmSwap })}</span>{/if}</dd>
    <dt>{t("dialogs.props.threadsFiles")}</dt>
    <dd>{t("dialogs.props.threads", { count: data.threads ?? 0, n: data.threads ?? "—" })}, {data.fds >= 0 ? t("dialogs.props.openFiles", { count: data.fds }) : t("dialogs.props.filesNotAccessible")}</dd>
    <dt>{t("common.priority")}</dt>
    <dd>{prio}<span class="soft"> (nice {data.nice})</span></dd>
    {#if data.unit}
      <dt>{t("dialogs.props.unit")}</dt>
      <dd>{data.unit}</dd>
    {/if}
    <dt>Cgroup</dt>
    <dd class="cg">{data.cgroup}</dd>
  </dl>
  {#snippet footer()}
    <button class="btn" data-autofocus onclick={close}>{t("common.close")}</button>
  {/snippet}
</Dialog>

<style>
  dl {
    display: grid;
    grid-template-columns: 150px minmax(0, 1fr);
    gap: 10px 16px;
    margin: 6px 0 0;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    color: var(--text);
    overflow-wrap: anywhere;
    line-height: 1.45;
    user-select: text;
  }
  .path {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }
  .path span {
    flex: 1;
    min-width: 0;
  }
  .cmd {
    max-height: 7.2em;
    overflow: auto;
    color: var(--text-2);
  }
  .path button {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: var(--r-sm);
    color: var(--muted);
    flex: none;
  }
  .path button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .soft {
    color: var(--muted);
  }
  .cg {
    color: var(--text-2);
    font-size: var(--fs-sm);
  }
</style>

<script lang="ts">
  import { app } from "../lib/state.svelte";
  import { act } from "../lib/api";
  import { num, rate, sizeStr } from "../lib/format";
  import { procMenu } from "../lib/procmenu";
  import type { Col } from "../lib/table";
  import type { Proc, Section } from "../lib/types";
  import DataTable from "../components/DataTable.svelte";
  import AppIcon from "../components/AppIcon.svelte";
  import Icon from "../components/Icon.svelte";
  import ContextMenu, { type MenuItem } from "../components/ContextMenu.svelte";
  import { t } from "../lib/i18n.svelte";

  // All processes in a flat list, like the Windows Details tab.
  type Row = Proc & { gicon: string | null; section: Section; group: string };

  let selected = $state<string | null>(null);
  let sortKey = $state("cpu");
  let sortDir = $state<1 | -1>(-1);
  let menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);

  const all = $derived<Row[]>(
    (app.snap?.groups ?? []).flatMap((g) => g.procs.map((p) => ({ ...p, gicon: p.icon ?? g.icon, section: g.section, group: g.label }))),
  );
  const needle = $derived(app.search.trim().toLowerCase());
  const rows = $derived(
    needle
      ? all.filter((p) => p.name.toLowerCase().includes(needle) || String(p.pid) === needle || p.user.toLowerCase() === needle || (needle.length >= 3 && p.cmd.toLowerCase().includes(needle)))
      : all,
  );
  const key = (p: Proc) => `p:${p.pid}:${p.ctime}`;
  const sel = $derived(rows.find((r) => key(r) === selected) ?? null);
  const prio = (n: number) => t(n >= 19 ? "priority.efficiency" : n > 0 ? "priority.low" : n < 0 ? "priority.high" : "priority.normal");
</script>

{#snippet name(p: Row)}
  <AppIcon src={p.gicon} section={p.section} size={18} />
  <span class="t">{p.name}</span>
{/snippet}
{#snippet pid(p: Row)}{p.pid}{/snippet}
{#snippet status(p: Row)}
  <span class="dot-status" class:ok={p.status === "running"} class:warn={p.status !== "running"}><i></i>{t(`processes.status.${p.status}`)}</span>
{/snippet}
{#snippet user(p: Row)}<span class="soft t">{p.user}</span>{/snippet}
{#snippet cpu(p: Row)}{p.cpu < 0.05 ? "0" : num(p.cpu, 1)}<small>%</small>{/snippet}
{#snippet mem(p: Row)}{sizeStr(p.mem)}{/snippet}
{#snippet disk(p: Row)}
  {#if p.disk === null}<span class="faint" title={t("common.notReadable")}>—</span>{:else}{@const r = rate(p.disk)}{r[0]}<small>{r[1]}</small>{/if}
{/snippet}
{#snippet threads(p: Row)}{p.threads}{/snippet}
{#snippet nice(p: Row)}<span class="soft">{prio(p.nice)}</span>{/snippet}
{#snippet group(p: Row)}<span class="soft t">{p.group}</span>{/snippet}
{#snippet cmd(p: Row)}<span class="faint t" title={p.cmd}>{p.cmd}</span>{/snippet}

<div class="pg">
  <div class="pg-bar">
    <span class="note">{t(needle ? "details.noteFound" : "details.note", { count: rows.length, n: num(rows.length, 0), threads: num(rows.reduce((a, p) => a + p.threads, 0), 0) })}</span>
    <div class="pg-actions">
      <button class="btn" disabled={!sel} onclick={() => sel && act("properties", { pid: sel.pid, ctime: sel.ctime })}><Icon name="info" size={14} />{t("menu.properties")}</button>
      <button class="btn" disabled={!sel} onclick={() => sel && act("signal", { pid: sel.pid, ctime: sel.ctime, sig: "TERM" })}><Icon name="x" size={14} />{t("menu.endTask")}</button>
    </div>
  </div>
  <div class="pg-panel">
    <DataTable
      label={t("details.label")}
      {rows}
      rowKey={key}
      bind:selected
      bind:sortKey
      bind:sortDir
      empty={needle ? t("details.noMatch", { query: app.search }) : t("details.empty")}
      onmenu={(e, p) => (menu = { x: e.clientX, y: e.clientY, items: procMenu(p) })}
      onopen={(p) => act("properties", { pid: p.pid, ctime: p.ctime })}
      onkey={(e, p) => {
        if (e.key === "Delete") act("signal", { pid: p.pid, ctime: p.ctime, sig: e.shiftKey ? "KILL" : "TERM" });
      }}
      cols={[
        { key: "name", label: t("common.name"), width: "minmax(160px, 1.2fr)", sort: (p) => p.name.toLowerCase(), cell: name },
        { key: "pid", label: "PID", width: "70px", align: "right", sort: (p) => p.pid, cell: pid },
        { key: "status", label: t("common.status"), width: "118px", sort: (p) => p.status, cell: status },
        { key: "user", label: t("common.user"), width: "92px", sort: (p) => p.user, cell: user },
        { key: "cpu", label: "CPU", width: "66px", align: "right", sort: (p) => p.cpu, cell: cpu },
        { key: "mem", label: t("common.memory"), width: "88px", align: "right", sort: (p) => p.mem, cell: mem },
        { key: "disk", label: t("common.disk"), width: "84px", align: "right", sort: (p) => p.disk ?? -1, cell: disk },
        { key: "threads", label: t("common.threads"), width: "62px", align: "right", sort: (p) => p.threads, cell: threads },
        { key: "nice", label: t("common.priority"), width: "84px", sort: (p) => p.nice, cell: nice },
        { key: "group", label: t("details.belongsTo"), width: "minmax(90px, 0.8fr)", sort: (p) => p.group.toLowerCase(), cell: group },
        { key: "cmd", label: t("common.commandLine"), width: "minmax(100px, 1.4fr)", cell: cmd },
      ] satisfies Col<Row>[]}
    />
  </div>
</div>
{#if menu}<ContextMenu {...menu} onclose={() => (menu = null)} />{/if}

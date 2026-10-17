<script lang="ts">
  import { onMount } from "svelte";
  import { app } from "../lib/state.svelte";
  import { invoke } from "../lib/api";
  import { num, rate, sizeStr } from "../lib/format";
  import type { Col } from "../lib/table";
  import type { Group } from "../lib/types";
  import DataTable from "../components/DataTable.svelte";
  import AppIcon from "../components/AppIcon.svelte";

  // Consumo per utente; sotto, le app dell'utente selezionato.
  type U = { name: string; procs: number; cpu: number; mem: number; disk: number; gpu: number; groups: Group[] };

  let me = $state("");
  let selected = $state<string | null>(null);
  let sortKey = $state("cpu");
  let sortDir = $state<1 | -1>(-1);
  let gsel = $state<string | null>(null);
  let gsort = $state("cpu");
  let gdir = $state<1 | -1>(-1);

  onMount(() => {
    invoke<{ user: string }>("session_info", {}, () => ({ user: "mario" })).then((i) => (me = i.user), () => {});
  });

  const users = $derived.by(() => {
    const m = new Map<string, U>();
    for (const g of app.snap?.groups ?? []) {
      const owners = new Map<string, number>();
      for (const p of g.procs) {
        let u = m.get(p.user);
        if (!u) m.set(p.user, (u = { name: p.user, procs: 0, cpu: 0, mem: 0, disk: 0, gpu: 0, groups: [] }));
        u.procs++;
        u.cpu += p.cpu;
        u.mem += p.mem;
        u.disk += p.disk ?? 0;
        u.gpu += p.gpu;
        owners.set(p.user, (owners.get(p.user) ?? 0) + 1);
      }
      const top = [...owners].sort((a, b) => b[1] - a[1])[0]?.[0];
      if (top) m.get(top)!.groups.push(g);
    }
    return [...m.values()];
  });
  $effect(() => {
    if (!selected && me && users.some((u) => u.name === me)) selected = me;
  });
  const cur = $derived(users.find((u) => u.name === selected) ?? null);
  const initials = (n: string) => n.slice(0, 2).toUpperCase();
  const label = (n: string) => (n === me ? `${n} (tu)` : n === "root" ? "root (sistema)" : n);
</script>

{#snippet uname(u: U)}
  <span class="av" class:me={u.name === me}>{initials(u.name)}</span>
  <span class="t">{label(u.name)}</span>
{/snippet}
{#snippet procs(u: U)}{num(u.procs, 0)}{/snippet}
{#snippet ucpu(u: U)}{u.cpu < 0.05 ? "0" : num(u.cpu, 1)}<small>%</small>{/snippet}
{#snippet umem(u: U)}{sizeStr(u.mem)}{/snippet}
{#snippet udisk(u: U)}{@const r = rate(u.disk)}{r[0]}<small>{r[1]}</small>{/snippet}
{#snippet ugpu(u: U)}{u.gpu < 0.05 ? "0" : num(Math.min(100, u.gpu), 1)}<small>%</small>{/snippet}

{#snippet gname(g: Group)}
  <AppIcon src={g.icon} section={g.section} size={18} />
  <span class="t">{g.label}</span>
  {#if g.procs.length > 1}<span class="faint">{g.procs.length}</span>{/if}
{/snippet}
{#snippet gcpu(g: Group)}{g.cpu < 0.05 ? "0" : num(g.cpu, 1)}<small>%</small>{/snippet}
{#snippet gmem(g: Group)}{sizeStr(g.mem)}{/snippet}
{#snippet gdisk(g: Group)}{#if g.disk === null}<span class="faint">—</span>{:else}{@const r = rate(g.disk)}{r[0]}<small>{r[1]}</small>{/if}{/snippet}
{#snippet ggpu(g: Group)}{g.gpu < 0.05 ? "0" : num(g.gpu, 1)}<small>%</small>{/snippet}

<div class="pg">
  <div class="pg-bar">
    <span class="note">{users.length} utenti con processi attivi</span>
  </div>
  <div class="pg-panel top">
    <DataTable
      label="Utenti"
      rows={users}
      rowKey={(u) => u.name}
      bind:selected
      bind:sortKey
      bind:sortDir
      cols={[
        { key: "name", label: "Utente", width: "minmax(200px, 1fr)", sort: (u) => u.name, cell: uname },
        { key: "procs", label: "Processi", width: "96px", align: "right", sort: (u) => u.procs, cell: procs },
        { key: "cpu", label: "CPU", width: "88px", align: "right", sort: (u) => u.cpu, cell: ucpu },
        { key: "mem", label: "Memoria", width: "108px", align: "right", sort: (u) => u.mem, cell: umem },
        { key: "disk", label: "Disco", width: "100px", align: "right", sort: (u) => u.disk, cell: udisk },
        { key: "gpu", label: "GPU", width: "80px", align: "right", sort: (u) => u.gpu, cell: ugpu },
      ] satisfies Col<U>[]}
    />
  </div>
  <h3>{cur ? `App e servizi di ${label(cur.name)}` : "Seleziona un utente"}</h3>
  <div class="pg-panel">
    <DataTable
      label="App dell'utente"
      rows={cur?.groups ?? []}
      rowKey={(g) => g.key}
      bind:selected={gsel}
      bind:sortKey={gsort}
      bind:sortDir={gdir}
      onopen={(g) => {
        app.selected = g.key;
        app.page = "processes";
      }}
      empty="Nessuna app per questo utente"
      cols={[
        { key: "name", label: "Nome", width: "minmax(200px, 1fr)", sort: (g) => g.label.toLowerCase(), cell: gname },
        { key: "cpu", label: "CPU", width: "88px", align: "right", sort: (g) => g.cpu, cell: gcpu },
        { key: "mem", label: "Memoria", width: "108px", align: "right", sort: (g) => g.mem, cell: gmem },
        { key: "disk", label: "Disco", width: "100px", align: "right", sort: (g) => g.disk ?? -1, cell: gdisk },
        { key: "gpu", label: "GPU", width: "80px", align: "right", sort: (g) => g.gpu, cell: ggpu },
      ] satisfies Col<Group>[]}
    />
  </div>
</div>

<style>
  .top {
    flex: 0 0 auto;
    height: min(38%, 300px);
  }
  h3 {
    margin: 4px 0 -4px;
    font-size: var(--fs-md);
    font-weight: 600;
  }
  .av {
    display: grid;
    place-items: center;
    flex: none;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    font-size: 10px;
    font-weight: 600;
    color: var(--text-2);
    background: var(--raised);
    border: 1px solid var(--line);
  }
  .av.me {
    color: var(--on-text);
    background: var(--text);
    border-color: var(--text);
  }
</style>

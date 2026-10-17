<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "../lib/api";
  import { app } from "../lib/state.svelte";
  import { bits, bitsStr, num } from "../lib/format";
  import { mockNet } from "../lib/mockpages";
  import type { Col } from "../lib/table";
  import type { NetApp } from "../lib/types";
  import DataTable from "../components/DataTable.svelte";
  import AppIcon from "../components/AppIcon.svelte";
  import Trace from "../components/Trace.svelte";
  import { t } from "../lib/i18n.svelte";

  // Who is using the network now: per-app speed from TCP/UDP sockets, plus interface status.
  let rows = $state<NetApp[]>([]);
  let loaded = $state(false);
  let selected = $state<string | null>(null);
  let sortKey = $state("total");
  let sortDir = $state<1 | -1>(-1);

  async function load() {
    try {
      rows = await invoke<NetApp[]>("net_apps", {}, mockNet);
    } catch {}
    loaded = true;
  }
  onMount(() => {
    load();
    const t = setInterval(load, 2000);
    return () => clearInterval(t);
  });

  const nets = $derived((app.snap?.nets ?? []).filter((n) => n.connected));
  const needle = $derived(app.search.trim().toLowerCase());
  const shown = $derived(rows.filter((r) => !needle || r.label.toLowerCase().includes(needle) || r.remotes.some((x) => x.includes(needle))));
  const ifaceTotal = $derived(nets.filter((n) => n.kind !== "VPN").reduce((a, n) => a + n.recv + n.send, 0));
  const appTotal = $derived(rows.reduce((a, r) => a + r.recv + r.send, 0));
  const hidden = $derived(ifaceTotal > 64 * 1024 && appTotal < ifaceTotal * 0.6);
  const kind = (k: string) => (k === "Wi-Fi" ? "Wi-Fi" : k === "VPN" ? "VPN" : k);
</script>

{#snippet name(r: NetApp)}
  <AppIcon src={r.icon} section={r.section} size={18} />
  <span class="t">{r.label}</span>
{/snippet}
{#snippet recv(r: NetApp)}{@const b = bits(r.recv)}{b[0]}<small>{b[1]}</small>{/snippet}
{#snippet send(r: NetApp)}{@const b = bits(r.send)}{b[0]}<small>{b[1]}</small>{/snippet}
{#snippet conns(r: NetApp)}{num(r.conns, 0)}{/snippet}
{#snippet remotes(r: NetApp)}<span class="faint t" title={r.remotes.join("\n")}>{r.remotes.join(", ")}</span>{/snippet}

<div class="pg">
  <div class="ifaces">
    {#each nets as n (n.name)}
      <div class="iface">
        <div class="ihead">
          <span class="iname">{n.ssid ?? kind(n.kind)}</span>
          <span class="faint">{n.ssid ? `${n.kind}, ` : ""}{n.name}{n.link ? `, ${t("network.link", { speed: bitsStr(n.link / 8) })}` : ""}</span>
        </div>
        <div class="rates">
          <span><span class="arrow">↓</span>{bitsStr(n.recv)}</span>
          <span><span class="arrow">↑</span>{bitsStr(n.send)}</span>
        </div>
        <div class="tr">
          <Trace samples={app.hist[`netR:${n.name}`] ?? []} second={app.hist[`netS:${n.name}`] ?? []} now={app.snap?.t ?? 0} max="auto" floor={125000} color="var(--net)" graticule="none" />
        </div>
      </div>
    {:else}
      <p class="soft">{t("network.none")}</p>
    {/each}
  </div>
  {#if hidden}
    <p class="hint">{t("network.hint")}</p>
  {/if}
  <div class="pg-panel">
    <DataTable
      label={t("network.label")}
      rows={shown}
      rowKey={(r) => r.key}
      bind:selected
      bind:sortKey
      bind:sortDir
      empty={loaded ? t("network.empty") : t("network.loading")}
      cols={[
        { key: "name", label: t("network.app"), width: "minmax(220px, 1.2fr)", sort: (r) => r.label.toLowerCase(), cell: name },
        { key: "total", label: t("network.receive"), width: "120px", align: "right", sort: (r) => r.recv + r.send * 0.001, cell: recv },
        { key: "send", label: t("network.send"), width: "110px", align: "right", sort: (r) => r.send, cell: send },
        { key: "conns", label: t("network.connections"), width: "110px", align: "right", sort: (r) => r.conns, cell: conns },
        { key: "remotes", label: t("network.remote"), width: "minmax(200px, 1.4fr)", cell: remotes },
      ] satisfies Col<NetApp>[]}
    />
  </div>
</div>

<style>
  .hint {
    margin: -2px 0 -2px;
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .ifaces {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 12px;
  }
  .iface {
    display: grid;
    grid-template-columns: 1fr auto;
    grid-template-rows: auto auto;
    gap: 2px 12px;
    padding: 12px 14px 0;
    border-radius: var(--r-md);
    background: var(--panel-bg);
    border: 1px solid var(--panel-edge);
    box-shadow: var(--panel-shadow);
    overflow: hidden;
  }
  .ihead {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .iname {
    font-weight: 600;
  }
  .ihead .faint {
    font-size: var(--fs-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rates {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
    font-stretch: 88%;
  }
  .arrow {
    margin-right: 5px;
    color: var(--faint);
  }
  .tr {
    grid-column: 1 / -1;
    height: 40px;
    margin: 6px -14px 0;
  }
</style>

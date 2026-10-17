<script lang="ts">
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
    started.toLocaleString("it-IT", { weekday: "long", day: "numeric", month: "long", hour: "2-digit", minute: "2-digit", second: "2-digit" }),
  );
  const prio = $derived(data.nice == null ? "" : data.nice >= 19 ? "Minima (efficienza)" : data.nice > 0 ? "Bassa" : data.nice < 0 ? "Alta" : "Normale");

  function copy(v: string) {
    navigator.clipboard?.writeText(v);
    ui.toast("Copiato negli appunti");
  }
</script>

<Dialog title={data.group && data.group !== data.name ? `${data.name}, in ${data.group}` : data.name} onclose={close} width={600}>
  <dl>
    <dt>Programma</dt>
    <dd class="path">
      <span>{data.exe || "Non accessibile"}</span>
      {#if data.exe}<button aria-label="Apri percorso" title="Apri percorso" onclick={() => act("open-location", { pid: data.pid })}><Icon name="folder" size={14} /></button>{/if}
    </dd>
    <dt>Riga di comando</dt>
    <dd class="path">
      <span class="cmd">{data.cmd || "—"}</span>
      {#if data.cmd}<button aria-label="Copia" title="Copia" onclick={() => copy(data.cmd)}><Icon name="copy" size={14} /></button>{/if}
    </dd>
    <dt>Cartella di lavoro</dt>
    <dd>{data.cwd || "Non accessibile"}</dd>
    <dt>Avviato</dt>
    <dd>{when}, {since} fa</dd>
    <dt>PID</dt>
    <dd>{data.pid}{#if data.ppid}<span class="soft">, figlio di {data.ppid}</span>{/if}</dd>
    <dt>Memoria fisica</dt>
    <dd>{data.vmRss || "—"}{#if data.vmSwap && data.vmSwap !== "0 kB"}<span class="soft">, più {data.vmSwap} nello swap</span>{/if}</dd>
    <dt>Thread e file aperti</dt>
    <dd>{data.threads ?? "—"} thread, {data.fds >= 0 ? `${data.fds} file aperti` : "file non accessibili"}</dd>
    <dt>Priorità</dt>
    <dd>{prio}<span class="soft"> (nice {data.nice})</span></dd>
    {#if data.unit}
      <dt>Unità systemd</dt>
      <dd>{data.unit}</dd>
    {/if}
    <dt>Cgroup</dt>
    <dd class="cg">{data.cgroup}</dd>
  </dl>
  {#snippet footer()}
    <button class="btn" data-autofocus onclick={close}>Chiudi</button>
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

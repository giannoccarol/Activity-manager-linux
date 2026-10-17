<script lang="ts">
  import { app } from "../lib/state.svelte";
  import { act } from "../lib/api";
  import type { Insight } from "../lib/types";
  import Icon from "./Icon.svelte";
  import { slide } from "svelte/transition";
  import { easeOut, rise, reduced } from "../lib/motion.svelte";
  import { t } from "../lib/i18n.svelte";

  let { onshow }: { onshow?: (key: string) => void } = $props();

  let index = $state(0);
  let hidden = $state<Record<string, number>>({});

  const list = $derived(
    (app.snap?.insights ?? []).filter((i) => (hidden[keyOf(i)] ?? 0) < Date.now()),
  );
  const cur = $derived(list[Math.min(index, list.length - 1)]);

  function keyOf(i: Insight) {
    return `${i.res}:${i.culprit?.key ?? "-"}`;
  }

  function parts(i: Insight): [string, string, string] {
    const name = i.culprit?.label;
    const at = name ? i.sentence.indexOf(name) : -1;
    if (!name || at < 0) return [i.sentence, "", ""];
    return [i.sentence.slice(0, at), name, i.sentence.slice(at + name.length)];
  }

  function hide() {
    if (!cur) return;
    hidden = { ...hidden, [keyOf(cur)]: Date.now() + 15 * 60_000 };
    index = 0;
  }

  const icon = (r: string) => ({ disk: "folder", mem: "processes", cpu: "performance", temp: "power", gpu: "diamond", net: "network" })[r] ?? "info";
</script>

{#if cur}
  {@const [a, b, c] = parts(cur)}
  <div class="wrap" transition:slide={{ duration: reduced() ? 0 : 280, easing: easeOut }}>
  <section class="diag {cur.level}" aria-live="polite">
    <span class="mark"><Icon name={icon(cur.res)} size={18} stroke={1.8} /></span>
    {#key keyOf(cur)}
    <div class="body" in:rise={{ y: 4, duration: 220 }}>
      <p class="title">{cur.title}</p>
      <p class="sentence">{a}<strong>{b}</strong>{c}</p>
      {#if cur.others.length}
        <p class="others">{t("diagnosis.others", { list: cur.others.join(", ") })}</p>
      {/if}
    </div>
    {/key}
    <div class="actions">
      {#if cur.culprit}
        <button class="btn" onclick={() => onshow?.(cur.culprit!.key)}>{t("diagnosis.show")}</button>
        {#if cur.culprit.container}
          <button class="btn" onclick={() => act("container", { verb: "pause", key: cur.culprit!.key })}>
            <Icon name="pause" size={14} />{t("diagnosis.pause")}
          </button>
          <button class="btn" onclick={() => act("container", { verb: "stop", key: cur.culprit!.key })}>
            <Icon name="stop" size={14} />{t("diagnosis.stop")}
          </button>
        {:else}
          <button class="btn" onclick={() => act("efficiency", { key: cur.culprit!.key, on: true })}>
            <Icon name="leaf" size={14} />{t("menu.efficiency")}
          </button>
          <button class="btn" onclick={() => act("terminate", { key: cur.culprit!.key })}>{t("diagnosis.end")}</button>
        {/if}
      {/if}
    </div>
    <div class="side">
      {#if list.length > 1}
        <span class="pager">
          <button aria-label={t("diagnosis.prev")} onclick={() => (index = (index - 1 + list.length) % list.length)}>
            <Icon name="chev-up" size={14} />
          </button>
          <span>{t("diagnosis.pager", { index: Math.min(index, list.length - 1) + 1, total: list.length })}</span>
          <button aria-label={t("diagnosis.next")} onclick={() => (index = (index + 1) % list.length)}>
            <Icon name="chev-down" size={14} />
          </button>
        </span>
      {/if}
      <button class="close" aria-label={t("diagnosis.hide")} title={t("diagnosis.hide")} onclick={hide}>
        <Icon name="x" size={14} />
      </button>
    </div>
  </section>
  </div>
{/if}

<style>
  .wrap {
    flex: none;
  }
  .diag {
    --s: var(--warn);
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto auto;
    gap: 14px;
    align-items: center;
    padding: 12px 12px 12px 16px;
    background: linear-gradient(90deg, color-mix(in srgb, var(--s) 10%, transparent), transparent 60%), var(--panel-bg);
    border: 1px solid var(--panel-edge);
    border-radius: var(--r-md);
    box-shadow: var(--panel-shadow);
  }
  .diag.critical {
    --s: var(--crit);
  }
  .mark {
    animation: mark-in var(--d-large) var(--ease-spring) both;
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    color: var(--s);
    background: color-mix(in srgb, var(--s) 14%, transparent);
    align-self: start;
  }
  @keyframes mark-in {
    from {
      transform: scale(0.6);
      opacity: 0;
    }
  }
  .body p {
    margin: 0;
  }
  .title {
    color: var(--s);
    font-weight: 600;
    font-size: var(--fs-md);
  }
  .sentence {
    color: var(--text);
    max-width: 78ch;
    margin-top: 2px !important;
  }
  .sentence strong {
    font-weight: 600;
  }
  .others {
    color: var(--muted);
    font-size: var(--fs-sm);
    margin-top: 4px !important;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 12px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line);
    background: var(--raised);
    color: var(--text);
    white-space: nowrap;
    transition: background-color 120ms, border-color 120ms;
  }
  .btn:hover {
    border-color: var(--faint);
  }
  .side {
    display: flex;
    align-items: center;
    gap: 6px;
    align-self: start;
  }
  .pager {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .pager button,
  .close {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: var(--r-sm);
    color: var(--muted);
  }
  .pager button:hover,
  .close:hover {
    background: var(--hover);
    color: var(--text);
  }
</style>

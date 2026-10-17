<script lang="ts" generics="T">
  import Icon from "./Icon.svelte";
  import { i18n, t } from "../lib/i18n.svelte";
  import type { Col } from "../lib/table";

  let {
    rows,
    cols,
    rowKey,
    label,
    selected = $bindable(null),
    sortKey = $bindable(""),
    sortDir = $bindable(-1),
    group,
    onmenu,
    onopen,
    onkey,
    empty,
    rowHeight = 36,
  }: {
    rows: T[];
    cols: Col<T>[];
    rowKey: (r: T) => string;
    label: string;
    selected?: string | null;
    sortKey?: string;
    sortDir?: 1 | -1;
    /** groups rows under headers (title, order) */
    group?: (r: T) => [string, number];
    onmenu?: (e: MouseEvent, r: T) => void;
    onopen?: (r: T) => void;
    onkey?: (e: KeyboardEvent, r: T) => void;
    empty?: string;
    /** fixed row height: needed to render only the visible ones */
    rowHeight?: number;
  } = $props();

  const HEAD = 38;
  const OVERSCAN = 8;
  let scroller: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let viewH = $state(600);

  const template = $derived(cols.map((c) => c.width).join(" "));

  function sortBy(c: Col<T>) {
    if (!c.sort) return;
    if (sortKey === c.key) sortDir = sortDir === 1 ? -1 : 1;
    else {
      sortKey = c.key;
      const sample = rows[0] ? c.sort(rows[0]) : 0;
      sortDir = typeof sample === "string" ? 1 : -1;
    }
  }

  const sorted = $derived.by(() => {
    const c = cols.find((c) => c.key === sortKey);
    if (!c?.sort) return rows;
    const f = c.sort;
    return [...rows].sort((a, b) => {
      const va = f(a);
      const vb = f(b);
      const r = typeof va === "string" && typeof vb === "string" ? va.localeCompare(vb, i18n.tag) : (va as number) - (vb as number);
      return r * sortDir;
    });
  });

  const sections = $derived.by(() => {
    if (!group) return [{ title: "", rows: sorted }];
    const m = new Map<string, { title: string; order: number; rows: T[] }>();
    for (const r of sorted) {
      const [title, order] = group(r);
      let s = m.get(title);
      if (!s) m.set(title, (s = { title, order, rows: [] }));
      s.rows.push(r);
    }
    return [...m.values()].sort((a, b) => a.order - b.order);
  });

  // flat list (section headers + rows) with the position of each item
  type Item = { head: true; title: string; count: number; top: number } | { head: false; r: T; k: string; top: number };
  const flat = $derived.by(() => {
    const items: Item[] = [];
    let y = 0;
    for (const s of sections) {
      if (s.title && s.rows.length) {
        items.push({ head: true, title: s.title, count: s.rows.length, top: y });
        y += HEAD;
      }
      for (const r of s.rows) {
        items.push({ head: false, r, k: rowKey(r), top: y });
        y += rowHeight;
      }
    }
    return { items, height: y };
  });
  const rowsOnly = $derived(flat.items.filter((i): i is Extract<Item, { head: false }> => !i.head));

  const visible = $derived.by(() => {
    const items = flat.items;
    let lo = 0;
    let hi = items.length;
    const from = scrollTop - OVERSCAN * rowHeight;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (items[mid].top < from) lo = mid + 1;
      else hi = mid;
    }
    const start = Math.max(0, lo - 1);
    const to = scrollTop + viewH + OVERSCAN * rowHeight;
    let end = start;
    while (end < items.length && items[end].top < to) end++;
    return { list: items.slice(start, end), before: items[start]?.top ?? 0, after: flat.height - (items[end]?.top ?? flat.height) };
  });

  function reveal(k: string) {
    const it = rowsOnly.find((i) => i.k === k);
    if (!it || !scroller) return;
    if (it.top < scroller.scrollTop) scroller.scrollTop = it.top;
    else if (it.top + rowHeight > scroller.scrollTop + scroller.clientHeight) scroller.scrollTop = it.top + rowHeight - scroller.clientHeight;
    queueMicrotask(() => (scroller?.querySelector(`[data-k="${CSS.escape(k)}"]`) as HTMLElement | null)?.focus());
  }

  function keydown(e: KeyboardEvent, r: T) {
    onkey?.(e, r);
    if (e.defaultPrevented) return;
    if (e.key === "Enter") onopen?.(r);
    if (e.key === "ArrowDown" || e.key === "ArrowUp" || e.key === "Home" || e.key === "End") {
      e.preventDefault();
      const i = rowsOnly.findIndex((x) => x.k === rowKey(r));
      const n = e.key === "Home" ? 0 : e.key === "End" ? rowsOnly.length - 1 : Math.max(0, Math.min(rowsOnly.length - 1, i + (e.key === "ArrowDown" ? 1 : -1)));
      const next = rowsOnly[n];
      if (next) {
        selected = next.k;
        reveal(next.k);
      }
    }
  }
</script>

<div class="table" role="grid" aria-label={label} style:--cols={template}>
  <div class="head" role="row">
    {#each cols as c (c.key)}
      {#if c.sort}
        <button class="h" class:right={c.align === "right"} class:sorted={sortKey === c.key} role="columnheader" aria-sort={sortKey === c.key ? (sortDir === 1 ? "ascending" : "descending") : "none"} onclick={() => sortBy(c)}>
          {c.label}
          {#if sortKey === c.key}<Icon name={sortDir === 1 ? "chev-up" : "chev-down"} size={12} stroke={2} />{/if}
        </button>
      {:else}
        <span class="h" class:right={c.align === "right"} role="columnheader">{c.label}</span>
      {/if}
    {/each}
  </div>
  <div class="rows" data-scroll bind:this={scroller} bind:clientHeight={viewH} onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)} style:--rh="{rowHeight}px">
    {#if flat.items.length}
      <div style:height="{visible.before}px"></div>
      {#each visible.list as it (it.head ? `h:${it.title}` : it.k)}
        {#if it.head}
          <div class="section"><span>{it.title}</span><span class="count">{it.count}</span></div>
        {:else}
          {@const r = it.r}
          {@const k = it.k}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <div
            class="row"
            class:selected={selected === k}
            role="row"
            data-k={k}
            tabindex={selected === k ? 0 : -1}
            aria-selected={selected === k}
            onclick={() => (selected = k)}
            ondblclick={() => onopen?.(r)}
            oncontextmenu={(e) => {
              if (!onmenu) return;
              e.preventDefault();
              selected = k;
              onmenu(e, r);
            }}
            onkeydown={(e) => keydown(e, r)}>
            {#each cols as c (c.key)}
              <span class="cell" class:right={c.align === "right"} role="gridcell">{@render c.cell(r)}</span>
            {/each}
          </div>
        {/if}
      {/each}
      <div style:height="{visible.after}px"></div>
    {:else}
      <p class="empty">{empty ?? t("common.noItems")}</p>
    {/if}
  </div>
</div>

<style>
  .table {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .head,
  .row {
    display: grid;
    grid-template-columns: var(--cols);
    align-items: center;
  }
  .head {
    height: 38px;
    padding: 0 8px;
    border-bottom: 1px solid var(--line);
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .h {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 100%;
    padding: 0 8px;
    color: inherit;
    white-space: nowrap;
  }
  .h.right {
    justify-content: flex-end;
  }
  button.h:hover,
  .h.sorted {
    color: var(--text);
  }
  .rows {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 4px 4px 12px;
    overflow-anchor: none;
  }
  .section {
    display: flex;
    gap: 8px;
    align-items: baseline;
    height: 38px;
    padding: 14px 12px 0;
    font-weight: 600;
    color: var(--text-2);
  }
  .section .count {
    color: var(--faint);
    font-weight: 500;
  }
  .row {
    position: relative;
    height: var(--rh);
    contain: layout style;
    padding: 0 4px;
    border-radius: var(--r-sm);
    outline-offset: -2px;
    transition: background-color 120ms;
  }
  .row:hover {
    background: var(--hover);
  }
  .row.selected {
    background: var(--select);
  }
  .row.selected::before {
    content: "";
    position: absolute;
    left: 0;
    top: 9px;
    bottom: 9px;
    width: 2px;
    border-radius: 2px;
    background: var(--text);
    animation: bar-in var(--d-small) var(--ease-spring) both;
  }
  @keyframes bar-in {
    from {
      transform: scaleY(0.2);
      opacity: 0;
    }
  }
  .cell {
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 0;
    padding: 0 8px;
    overflow: hidden;
    white-space: nowrap;
  }
  .cell.right {
    justify-content: flex-end;
    font-stretch: 88%;
  }
  .empty {
    margin: 48px auto;
    max-width: 52ch;
    text-align: center;
    color: var(--muted);
  }
  /* utilities shared by the pages' cells */
  .cell :global(.t) {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .cell :global(.soft) {
    color: var(--muted);
  }
  .cell :global(.faint) {
    color: var(--faint);
  }
  .cell :global(small) {
    margin-left: 3px;
    color: var(--faint);
    font-size: var(--fs-xs);
    font-stretch: 100%;
  }
</style>

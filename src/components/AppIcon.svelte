<script lang="ts">
  import Icon from "./Icon.svelte";
  import type { Section } from "../lib/types";
  import { iconUrl } from "../lib/icons.svelte";

  // Icona di un'app o di un processo: file del tema, oppure un glifo ("glyph:nome") o quello della sezione.
  let { src, section = "bg", size = 20 }: { src?: string | null; section?: Section; size?: number } = $props();
  let broken = $state(false);
  $effect(() => {
    src;
    broken = false;
  });
  const url = $derived(iconUrl(src));
  // sfuma solo l'icona che arriva dopo (prima volta che si legge il file), non a ogni riga ricreata
  const late = !iconUrl(src);
  const glyph = $derived(src?.startsWith("glyph:") ? src.slice(6) : section === "ctr" ? "container" : section === "sys" ? "service" : section === "app" ? "app" : "terminal");
</script>

{#if url && !broken}
  <img src={url} alt="" class:late width={size} height={size} style:width="{size}px" style:height="{size}px" onerror={() => (broken = true)} />
{:else}
  <span class="glyph" style:width="{size}px" style:height="{size}px"><Icon name={glyph} size={Math.round(size * 0.7)} /></span>
{/if}

<style>
  img {
    flex: none;
    object-fit: contain;
  }
  img.late {
    animation: icon-in var(--d-small) var(--ease-out) both;
  }
  @keyframes icon-in {
    from {
      opacity: 0;
      transform: scale(0.85);
    }
  }
  .glyph {
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 5px;
    color: var(--muted);
    background: var(--raised);
    border: 1px solid var(--line-soft);
  }
</style>

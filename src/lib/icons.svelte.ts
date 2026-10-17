// Icone dei file locali. WebKitGTK rifiuta <img src="asset://…"> dalla pagina dell'app, ma fetch()
// funziona: ogni file viene letto una volta e servito come blob dalla memoria.
const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const urls = $state<Record<string, string>>({});
const pending = new Set<string>();

async function load(path: string) {
  pending.add(path);
  try {
    const { convertFileSrc } = await import("@tauri-apps/api/core");
    const r = await fetch(convertFileSrc(path));
    if (r.ok) urls[path] = URL.createObjectURL(await r.blob());
  } catch {}
}

/** URL mostrabile per un percorso di icona; null finché non è pronto. */
export function iconUrl(path?: string | null): string | null {
  if (!path || path.startsWith("glyph:")) return null;
  if (!isTauri) return path;
  const u = urls[path];
  if (u) return u;
  if (!pending.has(path)) load(path);
  return null;
}

// Icons from local files. WebKitGTK rejects <img src="asset://…"> from the app page, but fetch()
// works: each file is read once and served as a blob from memory.
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

/** Displayable URL for an icon path; null until it is ready. */
export function iconUrl(path?: string | null): string | null {
  if (!path || path.startsWith("glyph:")) return null;
  if (!isTauri) return path;
  const u = urls[path];
  if (u) return u;
  if (!pending.has(path)) load(path);
  return null;
}

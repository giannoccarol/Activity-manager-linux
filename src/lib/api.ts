// Collegamento al backend Rust (Tauri). Nel browser usa dati simulati per sviluppare l'interfaccia.
import type { Snapshot } from "./types";
import { ui, type Props } from "./ui.svelte";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function subscribe(cb: (s: Snapshot) => void): Promise<void> {
  if (isTauri) {
    const { listen } = await import("@tauri-apps/api/event");
    const { invoke } = await import("@tauri-apps/api/core");
    const { tickCost } = await import("./bench");
    await listen<Snapshot>("snapshot", (e) => {
      const t0 = performance.now();
      cb(e.payload);
      requestAnimationFrame(() => {
        tickCost.push(performance.now() - t0);
        if (tickCost.length > 60) tickCost.shift();
      });
    });
    await invoke("start_monitor");
    return;
  }
  const { mockSnapshot } = await import("./mock");
  // due minuti di storia già pronti, così i grafici sono pieni fin dall'apertura
  for (let s = 120; s > 0; s--) cb(mockSnapshot(s));
  cb(mockSnapshot(0));
  setInterval(() => cb(mockSnapshot()), 1000);
}

async function call(action: string, payload: Record<string, unknown>): Promise<string | null> {
  if (!isTauri) {
    console.info("[simulazione] azione", action, payload);
    return null;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("action", { action, payload });
}

/** Esegue un'azione sul backend; gli errori diventano un avviso in basso. */
export async function act(action: string, payload: Record<string, unknown> = {}): Promise<string | null> {
  if (action === "run-dialog") {
    ui.dialog = { kind: "run" };
    return null;
  }
  try {
    const r = await call(action, payload);
    if (action === "properties") {
      if (r) ui.dialog = { kind: "props", data: JSON.parse(r) as Props };
      else ui.toast("Le proprietà sono disponibili solo nell'app");
    } else if (r?.startsWith("Richiesta")) ui.toast(r);
    return r;
  } catch (e) {
    ui.toast(String(e), "crit");
    return null;
  }
}

/** Comando del backend; nel browser restituisce i dati simulati. */
export async function invoke<T>(cmd: string, args: Record<string, unknown> = {}, mock?: () => T): Promise<T> {
  if (!isTauri) {
    if (mock) return mock();
    throw new Error("Disponibile solo nell'app");
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export async function windowCmd(cmd: "minimize" | "toggleMaximize" | "close" | "drag"): Promise<void> {
  if (!isTauri) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const w = getCurrentWindow();
  if (cmd === "minimize") await w.minimize();
  else if (cmd === "toggleMaximize") await w.toggleMaximize();
  else if (cmd === "close") await w.close();
  else await w.startDragging();
}

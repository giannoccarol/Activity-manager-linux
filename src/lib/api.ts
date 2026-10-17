// Bridge to the Rust (Tauri) backend. In the browser it uses mock data to develop the UI.
import type { Snapshot } from "./types";
import { ui, type Props } from "./ui.svelte";
import { t } from "./i18n.svelte";

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
  // two minutes of history ready up front, so the charts are full from the start
  for (let s = 120; s > 0; s--) cb(mockSnapshot(s));
  cb(mockSnapshot(0));
  setInterval(() => cb(mockSnapshot()), 1000);
}

async function call(action: string, payload: Record<string, unknown>): Promise<string | null> {
  if (!isTauri) {
    console.info("[mock] action", action, payload);
    return null;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("action", { action, payload });
}

/** Runs an action on the backend; errors become a toast at the bottom. */
export async function act(action: string, payload: Record<string, unknown> = {}): Promise<string | null> {
  if (action === "run-dialog") {
    ui.dialog = { kind: "run" };
    return null;
  }
  try {
    const r = await call(action, payload);
    if (action === "properties") {
      if (r) ui.dialog = { kind: "props", data: JSON.parse(r) as Props };
      else ui.toast(t("toasts.propsOnlyInApp"));
    } else if (r && r !== "ok") ui.toast(r); // e.g. authorization requested for other users' processes
    return r;
  } catch (e) {
    ui.toast(String(e), "crit");
    return null;
  }
}

/** Backend command; in the browser it returns mock data. */
export async function invoke<T>(cmd: string, args: Record<string, unknown> = {}, mock?: () => T): Promise<T> {
  if (!isTauri) {
    if (mock) return mock();
    throw new Error(t("toasts.onlyInApp"));
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

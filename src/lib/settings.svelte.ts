// Settings: read from the backend, applied immediately in the UI (the theme also from localStorage, without flicker).
import { isTauri } from "./api";
import { ui } from "./ui.svelte";

export type Theme = "graphite" | "paper" | "glass";
export type Settings = {
  theme: Theme;
  intervalMs: number;
  alerts: boolean;
  tray: boolean;
  autostart: boolean;
  shortcuts: boolean;
};

const DEFAULTS: Settings = { theme: "graphite", intervalMs: 1000, alerts: true, tray: true, autostart: false, shortcuts: true };

function storedTheme(): Theme | null {
  try {
    const t = localStorage.getItem("ga.theme");
    return t === "graphite" || t === "paper" || t === "glass" ? t : null;
  } catch {
    return null;
  }
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

class Store {
  v = $state<Settings>({ ...DEFAULTS, theme: storedTheme() ?? DEFAULTS.theme });

  async init() {
    if (isTauri) {
      try {
        this.v = await invoke<Settings>("get_settings");
      } catch {}
    }
    this.remember();
  }

  remember() {
    try {
      localStorage.setItem("ga.theme", this.v.theme);
    } catch {}
  }

  async set(patch: Partial<Settings>) {
    const prev = this.v;
    this.v = { ...this.v, ...patch };
    this.remember();
    if (!isTauri) return;
    try {
      this.v = await invoke<Settings>("set_settings", { patch });
    } catch (e) {
      this.v = prev;
      this.remember();
      ui.toast(String(e), "crit");
    }
  }

}

export const settings = new Store();

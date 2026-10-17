// UI state (Svelte 5 runes) and value history for the charts.
// The snapshot and the history are $state.raw: they are replaced wholesale every second, without
// wrapping hundreds of processes in reactive proxies (that was the main cost of each update).
import type { Proc, Snapshot } from "./types";

export type Sample = [number, number];
const KEEP = 130; // ~2 minutes of samples

export type Page = "processes" | "performance" | "sensors" | "history" | "startup" | "network" | "users" | "details" | "services" | "settings";

type Static = Pick<Proc, "name" | "icon" | "user" | "exe" | "cmd">;

class AppState {
  snap = $state.raw<Snapshot | null>(null);
  hist = $state.raw<Record<string, Sample[]>>({});
  page = $state<Page>("processes");
  search = $state("");
  selected = $state<string | null>(null);
  count = $state(0);
  /** static process fields: the backend sends them only the first time */
  #statics = new Map<string, Static>();

  push(s: Snapshot) {
    const live = new Set<string>();
    for (const g of s.groups)
      for (const p of g.procs) {
        p.gpu ??= 0;
        p.gpuEngine ??= "";
        p.nice ??= 0;
        const k = `${p.pid}:${p.ctime}`;
        live.add(k);
        if (p.name) this.#statics.set(k, { name: p.name, icon: p.icon ?? null, user: p.user ?? "", exe: p.exe ?? "", cmd: p.cmd ?? "" });
        else {
          const st = this.#statics.get(k);
          if (st) Object.assign(p, st);
          else Object.assign(p, { name: "?", icon: null, user: "", exe: "", cmd: "" });
        }
      }
    if (this.#statics.size > live.size * 2 + 64) for (const k of this.#statics.keys()) if (!live.has(k)) this.#statics.delete(k);

    const h: Record<string, Sample[]> = { ...this.hist };
    const add = (k: string, v: number) => {
      const a = h[k] ?? [];
      h[k] = a.length >= KEEP ? [...a.slice(a.length - KEEP + 1), [s.t, v]] : [...a, [s.t, v]];
    };
    add("cpu", s.cpu.total);
    add("cpuKernel", s.cpu.kernel);
    s.cpu.perCore.forEach((v, i) => add(`core${i}`, v));
    add("mem", s.mem.pct);
    add("disk", Math.max(0, ...s.disks.map((d) => d.active)));
    for (const d of s.disks) {
      add(`disk:${d.name}`, d.active);
      add(`diskR:${d.name}`, d.read);
      add(`diskW:${d.name}`, d.write);
    }
    const nets = s.nets.filter((n) => n.connected && n.kind !== "VPN");
    add("net", nets.reduce((a, n) => a + n.recv + n.send, 0));
    for (const n of s.nets) {
      add(`netR:${n.name}`, n.recv);
      add(`netS:${n.name}`, n.send);
    }
    add("gpu", Math.max(0, ...s.gpus.map((g) => g.util)));
    for (const g of s.gpus) add(`gpu:${g.idx}`, g.util);
    this.hist = h;
    this.snap = s;
    this.count++;
  }
}

export const app = new AppState();

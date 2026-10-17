// Ephemeral UI state: short toasts and dialogs.
export type Toast = { id: number; text: string; level: "info" | "crit" };
export type Props = {
  pid: number;
  ppid: number | null;
  name: string;
  exe: string;
  cwd: string;
  cmd: string;
  started: number;
  threads: number | null;
  nice: number | null;
  uid: string;
  vmRss: string;
  vmSwap: string;
  fds: number;
  cgroup: string;
  group: string | null;
  unit: string | null;
};
export type Dialog = { kind: "run" } | { kind: "props"; data: Props } | null;

class Ui {
  toasts = $state<Toast[]>([]);
  dialog = $state<Dialog>(null);
  #n = 0;

  toast(text: string, level: Toast["level"] = "info") {
    const id = ++this.#n;
    this.toasts.push({ id, text, level });
    setTimeout(() => (this.toasts = this.toasts.filter((t) => t.id !== id)), level === "crit" ? 6000 : 3200);
  }
}

export const ui = new Ui();

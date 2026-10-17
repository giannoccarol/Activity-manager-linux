// In-app updates: state of the check and of the installation, shared by Settings and the sidebar.
import { invoke, isTauri } from "./api";
import { settings } from "./settings.svelte";
import { t } from "./i18n.svelte";
import { ui } from "./ui.svelte";

export type UpdateInfo = {
  current: string;
  latest: string;
  newer: boolean;
  name: string;
  notes: string;
  url: string;
  published: string;
  assetUrl: string;
  shaUrl: string;
  size: number;
  canInstall: boolean;
  reason: string | null;
};
type Phase = "idle" | "checking" | "uptodate" | "available" | "downloading" | "verifying" | "installing" | "error";

const mockInfo = (): UpdateInfo => ({
  current: "2.1.0",
  latest: "2.2.0",
  newer: true,
  name: "Activity Manager 2.2.0",
  notes: "### What's new\n- **In-app updates**: check, download and install new versions from Settings.\n- Faster startup and smaller updates.\n\n### Fixes\n- The tray menu follows the language.",
  url: "https://github.com/giannoccarol/Activity-manager-linux/releases",
  published: new Date().toISOString(),
  assetUrl: "",
  shaUrl: "",
  size: 4_300_000,
  canInstall: true,
  reason: null,
});

class Updates {
  current = $state("");
  info = $state<UpdateInfo | null>(null);
  phase = $state<Phase>("idle");
  progress = $state(0);
  error = $state<string | null>(null);
  checkedAt = $state<Date | null>(null);
  #announced = "";

  /** a newer version the user has not chosen to ignore */
  get available() {
    return !!this.info?.newer && this.info.latest !== settings.v.skipVersion;
  }

  async init() {
    this.current = await invoke<string>("app_version", {}, () => "2.1.0").catch(() => "");
    if (!isTauri) return;
    const { listen } = await import("@tauri-apps/api/event");
    listen<UpdateInfo>("update-checked", (e) => this.found(e.payload));
    listen<{ phase: string; done?: number; total?: number }>("update-progress", (e) => {
      const p = e.payload;
      if (p.phase === "download") {
        this.phase = "downloading";
        this.progress = p.total ? Math.min(1, (p.done ?? 0) / p.total) : 0;
      } else if (p.phase === "verify") this.phase = "verifying";
      else if (p.phase === "install") this.phase = "installing";
    });
  }

  found(info: UpdateInfo) {
    this.info = info;
    this.checkedAt = new Date();
    if (this.phase === "downloading" || this.phase === "verifying" || this.phase === "installing") return;
    this.phase = info.newer ? "available" : "uptodate";
    if (this.available && this.#announced !== info.latest) {
      this.#announced = info.latest;
      ui.toast(t("toasts.updateAvailable", { name: t("app.name"), version: info.latest }));
    }
  }

  async check() {
    this.phase = "checking";
    this.error = null;
    try {
      const info = await invoke<UpdateInfo>("check_update", {}, mockInfo);
      this.#announced = info.latest; // asked explicitly: no toast
      this.found(info);
    } catch (e) {
      this.error = String(e);
      this.phase = "error";
    }
  }

  async install() {
    this.phase = "downloading";
    this.progress = 0;
    this.error = null;
    try {
      await invoke("install_update", {}, () => null);
      // install.sh closes this instance and starts the new version
      this.phase = "installing";
    } catch (e) {
      this.error = String(e);
      this.phase = "error";
    }
  }
}

export const updates = new Updates();

/** Release notes (GitHub Markdown) to safe HTML: everything is escaped, then a few tags are added back. */
export function notesHtml(md: string): string {
  const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
  const inline = (s: string) =>
    esc(s)
      .replace(/!\[[^\]]*\]\([^)]*\)/g, "")
      .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
      .replace(/`([^`]+)`/g, "<code>$1</code>")
      .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
      .replace(/(^|[^*])\*([^*]+)\*/g, "$1<em>$2</em>");
  const out: string[] = [];
  let list = false;
  let code = false;
  for (const raw of md.replace(/\r/g, "").split("\n")) {
    const line = raw.trimEnd();
    if (line.startsWith("```")) {
      if (list) (out.push("</ul>"), (list = false));
      out.push(code ? "</pre>" : "<pre>");
      code = !code;
      continue;
    }
    if (code) {
      out.push(esc(line));
      continue;
    }
    const item = line.match(/^\s*[-*]\s+(.*)$/);
    if (item) {
      if (!list) (out.push("<ul>"), (list = true));
      out.push(`<li>${inline(item[1])}</li>`);
      continue;
    }
    if (list) (out.push("</ul>"), (list = false));
    const h = line.match(/^#{1,6}\s+(.*)$/);
    if (h) out.push(`<h4>${inline(h[1])}</h4>`);
    else if (/^-{3,}$/.test(line)) out.push("<hr>");
    else if (line.trim() && !/^!\[/.test(line.trim())) out.push(`<p>${inline(line)}</p>`);
  }
  if (list) out.push("</ul>");
  if (code) out.push("</pre>");
  return out.join("\n");
}

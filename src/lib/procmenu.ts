// Context menu for a single process (Details and Users pages).
import { act } from "./api";
import { ui } from "./ui.svelte";
import { t } from "./i18n.svelte";
import type { Proc } from "./types";
import type { MenuItem } from "../components/ContextMenu.svelte";

export function procMenu(p: Proc): MenuItem[] {
  const id = { pid: p.pid, ctime: p.ctime };
  return [
    { label: t("menu.endTask"), icon: "x", run: () => act("signal", { ...id, sig: "TERM" }) },
    { label: t("menu.forceEnd"), danger: true, run: () => act("signal", { ...id, sig: "KILL" }) },
    { sep: true },
    { label: t(p.status === "stopped" ? "menu.resume" : "menu.suspend"), icon: p.status === "stopped" ? "play" : "pause", run: () => act("suspend", { ...id, resume: p.status === "stopped" }) },
    { label: t("menu.efficiency"), icon: "leaf", checked: p.status === "efficiency", run: () => act("efficiency", { ...id, on: p.status !== "efficiency" }) },
    { sep: true },
    { label: t("menu.openLocation"), icon: "folder", run: () => act("open-location", id) },
    {
      label: t("menu.copyCommandLine"),
      icon: "copy",
      run: () => {
        navigator.clipboard?.writeText(p.cmd);
        ui.toast(t("toasts.commandLineCopied"));
      },
    },
    { label: t("menu.properties"), icon: "info", run: () => act("properties", id) },
  ];
}

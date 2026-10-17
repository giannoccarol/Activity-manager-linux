// Menu contestuale di un singolo processo (pagine Dettagli e Utenti).
import { act } from "./api";
import { ui } from "./ui.svelte";
import type { Proc } from "./types";
import type { MenuItem } from "../components/ContextMenu.svelte";

export function procMenu(p: Proc): MenuItem[] {
  const id = { pid: p.pid, ctime: p.ctime };
  return [
    { label: "Termina attività", icon: "x", run: () => act("signal", { ...id, sig: "TERM" }) },
    { label: "Termina forzatamente", danger: true, run: () => act("signal", { ...id, sig: "KILL" }) },
    { sep: true },
    { label: p.status === "stopped" ? "Riprendi" : "Sospendi", icon: p.status === "stopped" ? "play" : "pause", run: () => act("suspend", { ...id, resume: p.status === "stopped" }) },
    { label: "Modalità efficienza", icon: "leaf", checked: p.status === "efficiency", run: () => act("efficiency", { ...id, on: p.status !== "efficiency" }) },
    { sep: true },
    { label: "Apri percorso file", icon: "folder", run: () => act("open-location", id) },
    {
      label: "Copia riga di comando",
      icon: "copy",
      run: () => {
        navigator.clipboard?.writeText(p.cmd);
        ui.toast("Riga di comando copiata");
      },
    },
    { label: "Proprietà", icon: "info", run: () => act("properties", id) },
  ];
}

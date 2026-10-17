// Transizioni dell'interfaccia: brevi, solo opacità e trasformazioni, rispettano "riduci animazioni".
import type { TransitionConfig } from "svelte/transition";
import { cubicOut } from "svelte/easing";

const mq = typeof window !== "undefined" ? window.matchMedia("(prefers-reduced-motion: reduce)") : null;
export const reduced = () => !!mq?.matches;

/** Animazioni periodiche (a ogni aggiornamento): grafici che scorrono, numeri, barrette, riordino.
 *  Girano solo con la finestra in primo piano: ogni fotogramma in più costa, e chi guarda l'app di
 *  sfuggita su un altro schermo non nota la differenza. Gli interruttori servono anche a --bench. */
export const flags = $state({ glide: true, flip: true, tween: true, meterSteps: 20, focused: true });

if (typeof window !== "undefined") {
  const sync = () => {
    flags.focused = document.hasFocus();
    document.documentElement.classList.toggle("calm", !flags.focused);
  };
  window.addEventListener("focus", sync);
  window.addEventListener("blur", sync);
  sync();
}
/** vero se le animazioni periodiche possono girare */
export const ambient = () => flags.focused && !reduced();
const d = (ms: number) => (reduced() ? 0 : ms);

/** curva di arrivo morbido (come --ease-out) */
export const easeOut = (t: number) => 1 - Math.pow(1 - t, 4);

/** entrata/uscita con leggero spostamento verticale */
export function rise(_: Element, { y = 6, duration = 180, delay = 0 } = {}): TransitionConfig {
  return { duration: d(duration), delay: reduced() ? 0 : delay, easing: easeOut, css: (t, u) => `opacity:${t};transform:translateY(${u * y}px)` };
}

/** comparsa di un elemento galleggiante (menu, finestra) dal punto d'origine */
export function pop(_: Element, { scale = 0.96, duration = 160 } = {}): TransitionConfig {
  return { duration: d(duration), easing: easeOut, css: (t, u) => `opacity:${t};transform:scale(${1 - u * (1 - scale)})` };
}

/** dissolvenza semplice */
export function fadeq(_: Element, { duration = 140, delay = 0 } = {}): TransitionConfig {
  return { duration: d(duration), delay, easing: cubicOut, css: (t) => `opacity:${t}` };
}

/** uscita verso sinistra (riga terminata) */
export function away(_: Element, { duration = 200 } = {}): TransitionConfig {
  return { duration: d(duration), easing: (t) => t * t, css: (t, u) => `opacity:${t};transform:translateX(${-u * 16}px)` };
}

/** durata di animate:flip, 0 con "riduci animazioni" */
export const flipMs = () => (flags.flip && flags.focused ? d(280) : 0);

/** Entrata di una vista tenuta in memoria quando torna attiva (senza ricostruirla). */
export function enterOn(node: HTMLElement, active: boolean) {
  let was = active;
  return {
    update(now: boolean) {
      if (now && !was && !reduced())
        node.animate([{ opacity: 0, transform: "translateY(8px)" }, { opacity: 1, transform: "none" }], { duration: 260, easing: "cubic-bezier(0.22, 1, 0.36, 1)" });
      was = now;
    },
  };
}

/** Cambio di tema con dissolvenza dei colori (classe temporanea, nessun costo dopo). */
export function themeFade(apply: () => void) {
  if (reduced()) return apply();
  const html = document.documentElement;
  html.classList.add("theme-anim");
  apply();
  setTimeout(() => html.classList.remove("theme-anim"), 360);
}

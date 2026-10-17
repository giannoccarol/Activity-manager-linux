// UI transitions: short, opacity and transforms only, honoring "reduce motion".
import type { TransitionConfig } from "svelte/transition";
import { cubicOut } from "svelte/easing";

const mq = typeof window !== "undefined" ? window.matchMedia("(prefers-reduced-motion: reduce)") : null;
export const reduced = () => !!mq?.matches;

/** Periodic animations (on every update): scrolling charts, numbers, meter bars, reordering.
 *  They run only while the window is focused: every extra frame has a cost, and someone glancing at
 *  the app on another screen won't notice the difference. The switches are also used by --bench. */
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
/** true if periodic animations may run */
export const ambient = () => flags.focused && !reduced();
const d = (ms: number) => (reduced() ? 0 : ms);

/** soft-landing curve (like --ease-out) */
export const easeOut = (t: number) => 1 - Math.pow(1 - t, 4);

/** enter/exit with a slight vertical shift */
export function rise(_: Element, { y = 6, duration = 180, delay = 0 } = {}): TransitionConfig {
  return { duration: d(duration), delay: reduced() ? 0 : delay, easing: easeOut, css: (t, u) => `opacity:${t};transform:translateY(${u * y}px)` };
}

/** appearance of a floating element (menu, dialog) from its origin point */
export function pop(_: Element, { scale = 0.96, duration = 160 } = {}): TransitionConfig {
  return { duration: d(duration), easing: easeOut, css: (t, u) => `opacity:${t};transform:scale(${1 - u * (1 - scale)})` };
}

/** simple fade */
export function fadeq(_: Element, { duration = 140, delay = 0 } = {}): TransitionConfig {
  return { duration: d(duration), delay, easing: cubicOut, css: (t) => `opacity:${t}` };
}

/** exit to the left (terminated row) */
export function away(_: Element, { duration = 200 } = {}): TransitionConfig {
  return { duration: d(duration), easing: (t) => t * t, css: (t, u) => `opacity:${t};transform:translateX(${-u * 16}px)` };
}

/** animate:flip duration, 0 with "reduce motion" */
export const flipMs = () => (flags.flip && flags.focused ? d(280) : 0);

/** Entrance of a view kept in memory when it becomes active again (without rebuilding it). */
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

/** Theme change with a color crossfade (temporary class, no cost afterwards). */
export function themeFade(apply: () => void) {
  if (reduced()) return apply();
  const html = document.documentElement;
  html.classList.add("theme-anim");
  apply();
  setTimeout(() => html.classList.remove("theme-anim"), 360);
}

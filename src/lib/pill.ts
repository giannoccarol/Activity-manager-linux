// Indicator that slides under the active item of a group (menu item, segment).
// A single absolute element moved with transform: no layout recalculation around it.
export function pill(node: HTMLElement, opts: { selector?: string; axis?: "x" | "y" } = {}) {
  const selector = opts.selector ?? "button.on";
  const el = document.createElement("span");
  el.className = "pill instant";
  el.setAttribute("aria-hidden", "true");
  node.prepend(el);
  let first = true;

  function place() {
    const target = node.querySelector<HTMLElement>(selector);
    if (!target) {
      el.style.opacity = "0";
      return;
    }
    const nr = node.getBoundingClientRect();
    const tr = target.getBoundingClientRect();
    // coordinates within the container's content, even when it scrolls
    const x = tr.left - nr.left - node.clientLeft + node.scrollLeft;
    const y = tr.top - nr.top - node.clientTop + node.scrollTop;
    if (opts.axis === "y") {
      el.style.width = `${tr.width}px`;
      el.style.height = `${tr.height}px`;
      el.style.transform = `translate(${x}px, ${y}px)`;
    } else {
      el.style.width = `${tr.width}px`;
      el.style.height = `${tr.height}px`;
      el.style.transform = `translateX(${x}px)`;
      el.style.top = `${y}px`;
    }
    el.style.opacity = "1";
    if (first) {
      first = false;
      requestAnimationFrame(() => el.classList.remove("instant"));
    }
  }

  // reposition when the active item or the size changes
  const mo = new MutationObserver(() => place());
  mo.observe(node, { attributes: true, subtree: true, attributeFilter: ["class", "aria-selected", "aria-checked"], childList: true });
  const ro = new ResizeObserver(() => {
    el.classList.add("instant");
    place();
    requestAnimationFrame(() => el.classList.remove("instant"));
  });
  ro.observe(node);
  requestAnimationFrame(place);

  return {
    destroy() {
      mo.disconnect();
      ro.disconnect();
      el.remove();
    },
  };
}

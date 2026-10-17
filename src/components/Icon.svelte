<script lang="ts">
  // Consistent stroke icons (24 grid, 1.6 stroke).
  let { name, size = 16, stroke = 1.6 }: { name: string; size?: number; stroke?: number } = $props();

  const P: Record<string, string> = {
    processes: '<rect x="3.5" y="3.5" width="7" height="7" rx="1.5"/><rect x="13.5" y="3.5" width="7" height="7" rx="1.5"/><rect x="3.5" y="13.5" width="7" height="7" rx="1.5"/><rect x="13.5" y="13.5" width="7" height="7" rx="1.5"/>',
    performance: '<path d="M3 12.5h4l2.5-6 4.5 12 3-7.5 1.5 1.5H21"/>',
    history: '<path d="M3.5 12a8.5 8.5 0 1 0 2.5-6"/><path d="M3.5 4v4.5H8"/><path d="M12 7.5V12l3 2"/>',
    startup: '<path d="M4.2 18.5a9 9 0 1 1 15.6 0"/><path d="M12 14l4-4.2"/>',
    network: '<circle cx="12" cy="12" r="8.5"/><path d="M3.5 12h17M12 3.5c2.4 2.6 3.6 5.4 3.6 8.5s-1.2 5.9-3.6 8.5c-2.4-2.6-3.6-5.4-3.6-8.5s1.2-5.9 3.6-8.5z"/>',
    users: '<circle cx="9" cy="8" r="3.3"/><path d="M3 19.5c.6-3.3 3-5.3 6-5.3s5.4 2 6 5.3"/><path d="M15.6 5a3.2 3.2 0 0 1 0 6"/><path d="M17.8 14.6c1.8.6 3 2.3 3.4 4.9"/>',
    details: '<path d="M9 6.5h11M9 12h11M9 17.5h11"/><path d="M4.5 6.5h.01M4.5 12h.01M4.5 17.5h.01"/>',
    services: '<path d="M5 5.5h4.6a2.4 2.4 0 0 1 4.8 0H19v4.6a2.4 2.4 0 0 1 0 4.8V19H5z"/>',
    settings: '<circle cx="12" cy="12" r="3"/><path d="M12 2.8v2.4M12 18.8v2.4M2.8 12h2.4M18.8 12h2.4M5.5 5.5l1.7 1.7M16.8 16.8l1.7 1.7M5.5 18.5l1.7-1.7M16.8 7.2l1.7-1.7"/>',
    search: '<circle cx="10.5" cy="10.5" r="6.5"/><path d="M20 20l-4.6-4.6"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    x: '<path d="M6.5 6.5l11 11M17.5 6.5l-11 11"/>',
    leaf: '<path d="M11 20A7 7 0 0 1 9.8 6.1C15.5 5 17 4.5 19 2c1 2 2 4.2 2 8 0 5.5-4.8 10-10 10z"/><path d="M2.5 21c0-3 1.9-5.4 5.1-6 2.4-.5 4.9-2 5.9-3"/>',
    pause: '<path d="M8.5 5v14M15.5 5v14"/>',
    play: '<path d="M7 4.8v14.4l12-7.2z"/>',
    stop: '<rect x="6" y="6" width="12" height="12" rx="2"/>',
    "chev-down": '<path d="M6.5 9.5l5.5 5.5 5.5-5.5"/>',
    "chev-up": '<path d="M6.5 14.5L12 9l5.5 5.5"/>',
    "chev-right": '<path d="M9.5 6.5l5.5 5.5-5.5 5.5"/>',
    kebab: '<path d="M12 5.5h.01M12 12h.01M12 18.5h.01"/>',
    folder: '<path d="M3 7.5a2 2 0 0 1 2-2h4l2 2.5h8a2 2 0 0 1 2 2V17a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>',
    info: '<circle cx="12" cy="12" r="8.5"/><path d="M12 16.5v-5M12 8h.01"/>',
    copy: '<rect x="8.5" y="8.5" width="12" height="12" rx="2"/><path d="M15.5 8.5V5.5a2 2 0 0 0-2-2h-8a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h3"/>',
    container: '<path d="M12 2.8l8 4.6v9.2l-8 4.6-8-4.6V7.4z"/><path d="M4 7.4l8 4.6 8-4.6M12 12v9.2"/>',
    service: '<rect x="3.5" y="4.5" width="17" height="6" rx="1.5"/><rect x="3.5" y="13.5" width="17" height="6" rx="1.5"/><path d="M7 7.5h.01M7 16.5h.01"/>',
    thermometer: '<path d="M10 13.6V5a2 2 0 1 1 4 0v8.6a4 4 0 1 1-4 0z"/><path d="M12 10v6"/>',
    chip: '<rect x="6" y="6" width="12" height="12" rx="2"/><rect x="9.5" y="9.5" width="5" height="5" rx="1"/><path d="M9.5 3v3M14.5 3v3M9.5 18v3M14.5 18v3M3 9.5h3M3 14.5h3M18 9.5h3M18 14.5h3"/>',
    gpu: '<rect x="2.5" y="6.5" width="19" height="11" rx="2"/><circle cx="9" cy="12" r="2.6"/><path d="M15 10.2h3.2M15 13.8h3.2M5.5 17.5v2.5"/>',
    memory: '<rect x="3" y="7" width="18" height="8.5" rx="1.5"/><path d="M7 10v2.5M10.3 10v2.5M13.7 10v2.5M17 10v2.5M6 15.5V18M10 15.5V18M14 15.5V18M18 15.5V18"/>',
    disk: '<rect x="3.5" y="5" width="17" height="14" rx="2.5"/><path d="M7.5 15h.01M11 15h5.5"/>',
    board: '<rect x="3.5" y="3.5" width="17" height="17" rx="2"/><rect x="7.5" y="7.5" width="4" height="4" rx=".8"/><path d="M14.5 8h2.5M14.5 11h2.5M7.5 15.5h9"/>',
    battery: '<rect x="2.5" y="7" width="17" height="10" rx="2"/><path d="M21.5 10.5v3M6 10.5v3M9.5 10.5v3"/>',
    app: '<rect x="3.5" y="3.5" width="17" height="17" rx="4"/><path d="M3.5 8.5h17"/>',
    terminal: '<rect x="3.5" y="4.5" width="17" height="15" rx="2.5"/><path d="M7.5 10l2.6 2.2-2.6 2.2M12.5 14.6h4"/>',
    restart: '<path d="M20 11a8 8 0 1 0-2.3 6.1"/><path d="M20 4.5V11h-6.5"/>',
    bell: '<path d="M6 16.5V11a6 6 0 1 1 12 0v5.5l1.5 2h-15z"/><path d="M10 20.5a2 2 0 0 0 4 0"/>',
    diamond: '<path d="M12 6.5L17.5 12 12 17.5 6.5 12z"/>',
    lock: '<rect x="5" y="10.5" width="14" height="10" rx="2.5"/><path d="M8 10.5V8a4 4 0 1 1 8 0v2.5"/>',
    logout: '<path d="M14 4.5h3.5a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H14"/><path d="M10 8l-4 4 4 4M6 12h9.5"/>',
    power: '<path d="M12 3v8"/><path d="M6.4 6.6a8 8 0 1 0 11.2 0"/>',
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true">{@html P[name] ?? ""}</svg>

// Mock data for developing the UI in the browser (modeled on a real system).
import type { Group, Insight, Proc, Section, Snapshot } from "./types";
import { rateStr, sizeStr, pct0 } from "./format";

const GiB = 1024 ** 3;
const MiB = 1024 ** 2;

interface Def {
  label: string;
  icon?: string;
  section: Section;
  n: number;
  cpu: number;
  mem: number;
  disk?: number;
  gpu?: number;
  image?: string;
  names?: string[];
}

const DEFS: Def[] = [
  { label: "Claude", icon: "/mock-icons/claude.png", section: "app", n: 46, cpu: 4.2, mem: 1.96 * GiB, disk: 0.1 * MiB, gpu: 2.1, names: ["claude-desktop", "claude", "chrome_crashpad_handler", "cowork-linux-helper"] },
  { label: "Google Chrome", icon: "/mock-icons/chrome.png", section: "app", n: 40, cpu: 6.2, mem: 2.2 * GiB, disk: 0.7 * MiB, gpu: 9, names: ["chrome", "chrome_crashpad"] },
  { label: "OpenCode", icon: "/mock-icons/opencode.png", section: "app", n: 17, cpu: 0.6, mem: 350 * MiB, gpu: 2.4, names: ["opencode", "node"] },
  { label: "Teams for Linux", icon: "/mock-icons/teams.svg", section: "app", n: 11, cpu: 0.2, mem: 488 * MiB, names: ["teams-for-linux"] },
  { label: "GitTree", icon: "/mock-icons/gittree.png", section: "app", n: 7, cpu: 0.1, mem: 105 * MiB, names: ["gittree"] },
  { label: "Dolphin", icon: "/mock-icons/dolphin.svg", section: "app", n: 1, cpu: 0.0, mem: 92 * MiB, names: ["dolphin"] },
  { label: "Gestione attività", icon: "/mock-icons/app.svg", section: "app", n: 1, cpu: 1.2, mem: 64 * MiB, names: ["gestione-attivita"] },
  { label: "postgres-dev", section: "ctr", n: 6, cpu: 38, mem: 7.4 * GiB, disk: 865 * MiB, image: "postgres:16", names: ["postgres", "postgres: walwriter", "postgres: autovacuum"] },
  { label: "KDE Plasma Workspace", section: "bg", n: 1, cpu: 0.4, mem: 266 * MiB, gpu: 0.3, names: ["plasmashell"] },
  { label: "KDE Wayland Compositor", section: "bg", n: 3, cpu: 1.5, mem: 64 * MiB, gpu: 3.9, names: ["kwin_wayland", "Xwayland", "kwin_wayland_wrapper"] },
  { label: "PipeWire Multimedia Service", section: "bg", n: 3, cpu: 0.3, mem: 22 * MiB, names: ["pipewire", "wireplumber", "pipewire-pulse"] },
  { label: "D-Bus User Message Bus", section: "bg", n: 2, cpu: 0.0, mem: 2.7 * MiB, names: ["dbus-broker"] },
  { label: "KDE Daemon 6", section: "bg", n: 2, cpu: 0.0, mem: 11 * MiB, names: ["kded6"] },
  { label: "Applet di Remmina", section: "bg", n: 1, cpu: 0.0, mem: 6.1 * MiB, names: ["remmina"] },
  { label: "KDE Connect", section: "bg", n: 1, cpu: 0.0, mem: 18 * MiB, names: ["kdeconnectd"] },
  { label: "Baloo", section: "bg", n: 1, cpu: 0.0, mem: 31 * MiB, names: ["baloo_file"] },
  { label: "Docker Application Container Engine", section: "sys", n: 2, cpu: 0.4, mem: 96 * MiB, names: ["dockerd", "docker-proxy"] },
  { label: "containerd container runtime", section: "sys", n: 3, cpu: 0.2, mem: 54 * MiB, names: ["containerd", "containerd-shim"] },
  { label: "Network Manager", section: "sys", n: 1, cpu: 0.0, mem: 14 * MiB, names: ["NetworkManager"] },
  { label: "Journal Service", section: "sys", n: 1, cpu: 0.1, mem: 9 * MiB, disk: 0.2 * MiB, names: ["systemd-journald"] },
  { label: "Tailscale node agent", section: "sys", n: 1, cpu: 0.1, mem: 38 * MiB, names: ["tailscaled"] },
  { label: "Bluetooth service", section: "sys", n: 1, cpu: 0.0, mem: 3 * MiB, names: ["bluetoothd"] },
  { label: "Ananicy-Cpp", section: "sys", n: 1, cpu: 0.2, mem: 7 * MiB, names: ["ananicy-cpp"] },
  { label: "CUPS Scheduler", section: "sys", n: 1, cpu: 0.0, mem: 5 * MiB, names: ["cupsd"] },
  { label: "User Login Management", section: "sys", n: 1, cpu: 0.0, mem: 4 * MiB, names: ["systemd-logind"] },
];

let tick = 0;
const t0 = performance.now() / 1000;
const jitter = (base: number, amp: number) => Math.max(0, base * (1 + (Math.random() - 0.5) * amp));
const walks = new Map<string, number>();
function walk(key: string, base: number, amp = 0.35): number {
  const prev = walks.get(key) ?? base;
  const next = prev + (base - prev) * 0.35 + base * (Math.random() - 0.5) * amp;
  const v = Math.max(0, next);
  walks.set(key, v);
  return v;
}

function makeGroup(d: Def, i: number): Group {
  const procs: Proc[] = [];
  const share = (k: number) => (k === 0 ? 0.55 : 0.45 / Math.max(1, d.n - 1));
  for (let k = 0; k < d.n; k++) {
    const name = d.names?.[k % (d.names?.length ?? 1)] ?? d.label.toLowerCase();
    procs.push({
      pid: 2000 + i * 97 + k * 13,
      ctime: 1000 + i,
      name,
      user: d.section === "sys" || d.section === "ctr" ? "root" : "mario",
      cpu: walk(`${d.label}:${k}:cpu`, d.cpu * share(k)),
      mem: jitter(d.mem * share(k), 0.02),
      disk: d.section === "sys" || d.section === "ctr" ? null : jitter((d.disk ?? 0) * share(k), 0.6),
      gpu: walk(`${d.label}:${k}:gpu`, (d.gpu ?? 0) * share(k), 0.5),
      gpuEngine: d.gpu ? "3D" : "",
      status: "running",
      nice: 0,
      threads: 4 + ((k * 7) % 30),
      exe: `/usr/bin/${name}`,
      cmd: `/usr/bin/${name} --type=renderer`,
    });
  }
  const sum = (f: (p: Proc) => number) => procs.reduce((a, p) => a + f(p), 0);
  const disk = d.disk ? walk(`${d.label}:disk`, d.disk, 0.15) : d.section === "sys" ? 0 : sum((p) => p.disk ?? 0);
  return {
    key: `${d.section}:${d.label}`,
    label: d.label,
    icon: d.icon ?? null,
    section: d.section,
    unit: d.section === "ctr" ? "docker-60be07404a36.scope" : `${d.label}.service`,
    image: d.image,
    cpu: sum((p) => p.cpu),
    mem: sum((p) => p.mem),
    disk,
    gpu: Math.min(100, sum((p) => p.gpu)),
    gpuEngine: d.gpu ? "3D" : "",
    status: "running",
    procs,
  };
}

/** simulated time offset, to fill the chart history right away */
let shift = 0;

export function mockSnapshot(offset?: number): Snapshot {
  if (offset !== undefined) shift = offset;
  tick++;
  const groups = DEFS.map(makeGroup);
  const cpuTotal = Math.min(100, groups.reduce((a, g) => a + g.cpu, 0) + walk("cpu:other", 8));
  const memUsed = groups.reduce((a, g) => a + g.mem, 0) + 6.8 * GiB;
  const memTotal = 30.8 * GiB;
  const ctr = groups.find((g) => g.section === "ctr")!;
  const write = (ctr.disk ?? 0) + walk("disk:other", 4 * MiB);
  const read = walk("disk:read", 1.2 * MiB);
  const active = Math.min(100, 92 + Math.random() * 8);
  const temp = 99 + Math.random() * 3;
  const insights: Insight[] = [
    {
      res: "disk",
      level: "critical",
      title: `Disco al ${pct0(active)}%`,
      sentence: `${ctr.label} sta scrivendo ${rateStr(ctr.disk ?? 0)}: è il ${pct0(((ctr.disk ?? 0) / (read + write)) * 100)}% di tutto l'I/O del disco, e le altre app ne risentono.`,
      culprit: { key: ctr.key, label: ctr.label, kind: "container", container: true },
      others: ["Google Chrome 0,7 MB/s", "Journal Service 0,2 MB/s"],
    },
    {
      res: "temp",
      level: "critical",
      title: `CPU a ${pct0(temp)} °C`,
      sentence: `Il calore viene soprattutto da ${ctr.label}, che usa il ${pct0(ctr.cpu)}% della CPU. Metterlo in pausa la raffredda in pochi secondi.`,
      culprit: { key: ctr.key, label: ctr.label, kind: "container", container: true },
      others: [`Google Chrome ${pct0(groups[1].cpu)}%`, `Claude ${pct0(groups[0].cpu)}%`],
    },
  ];
  return {
    t: performance.now() / 1000 - t0 - shift,
    cpu: {
      total: cpuTotal,
      kernel: cpuTotal * 0.22,
      perCore: Array.from({ length: 22 }, (_, i) => Math.min(100, walk(`core${i}`, cpuTotal * (0.6 + (i % 5) * 0.2)))),
      freq: 2400 + Math.random() * 600,
      temp,
      model: "Intel Core Ultra 7 155H",
    },
    mem: { total: memTotal, used: memUsed, available: memTotal - memUsed, pct: (memUsed / memTotal) * 100, swapUsed: 4.4 * GiB, swapTotal: 30.8 * GiB, cached: 7.2 * GiB },
    disks: [{ name: "nvme0n1", model: "SK hynix HFS001TEJ9X162N", kind: "SSD NVMe", active, read, write, resp: 18 + Math.random() * 8, capacity: 954 * 1e9, system: true }],
    nets: [
      { name: "wlan0", kind: "Wi-Fi", connected: true, ssid: "Casa", send: walk("net:tx", 90e3), recv: walk("net:rx", 1.6e6), link: 540e6 / 8 },
      { name: "tailscale0", kind: "VPN", connected: true, ssid: null, send: walk("vpn:tx", 300), recv: walk("vpn:rx", 800), link: 0 },
    ],
    gpus: [
      { idx: 0, name: "Intel Arc Graphics", vendor: "intel", util: Math.min(100, walk("gpu", 14)), engines: { "3D": walk("gpu3d", 13), Video: walk("gpuv", 1) }, memUsed: 1.2 * GiB, memTotal: 15.4 * GiB, temp: null, suspended: false },
      { idx: 1, name: "NVIDIA RTX 500 Ada", vendor: "nvidia", util: 0, engines: {}, memUsed: 0, memTotal: 4 * GiB, temp: null, suspended: true },
    ],
    groups,
    insights,
    counts: { procs: 268, threads: 3240, handles: 20410 },
    uptime: 4 * 3600 + 12 * 60 + tick,
  };
}

export const sizeOf = sizeStr;

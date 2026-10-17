// Contratto dei dati tra il backend Rust e l'interfaccia (serializzato in camelCase).

export type Section = "app" | "ctr" | "bg" | "sys";
export type Status = "running" | "stopped" | "efficiency" | "paused";
export type Resource = "cpu" | "mem" | "disk" | "net" | "gpu";

export interface Proc {
  pid: number;
  ctime: number;
  name: string;
  icon?: string | null;
  user: string;
  cpu: number; // % della CPU totale
  mem: number; // byte (memoria privata)
  disk: number | null; // byte/s, null se non leggibile
  gpu: number; // %
  gpuEngine: string;
  status: Status;
  nice: number;
  threads: number;
  exe: string;
  cmd: string;
}

export interface Group {
  key: string;
  label: string;
  icon: string | null; // URL dell'icona già risolta
  section: Section;
  unit: string;
  image?: string; // immagine del container
  cpu: number;
  mem: number;
  disk: number | null;
  gpu: number;
  gpuEngine: string;
  status: Status;
  procs: Proc[];
}

export interface Disk {
  name: string;
  model: string;
  kind: string;
  active: number;
  read: number;
  write: number;
  resp: number;
  capacity: number;
  system: boolean;
}

export interface Net {
  name: string;
  kind: string;
  connected: boolean;
  ssid: string | null;
  send: number;
  recv: number;
  link: number; // byte/s, 0 se sconosciuta
}

export interface Gpu {
  idx: number;
  name: string;
  vendor: string;
  util: number;
  engines: Record<string, number>;
  memUsed: number;
  memTotal: number | null;
  temp: number | null;
  suspended: boolean;
}

export interface Culprit {
  key: string;
  label: string;
  kind: string; // "app", "container", …
  container: boolean;
}

export interface Insight {
  res: Resource | "temp";
  level: "warn" | "critical";
  title: string;
  sentence: string;
  culprit: Culprit | null;
  others: string[];
}

export interface Snapshot {
  t: number; // secondi (monotonico)
  cpu: { total: number; kernel: number; perCore: number[]; freq: number; temp: number | null; model: string };
  mem: { total: number; used: number; available: number; pct: number; swapUsed: number; swapTotal: number; cached: number };
  disks: Disk[];
  nets: Net[];
  gpus: Gpu[];
  groups: Group[];
  insights: Insight[];
  counts: { procs: number; threads: number; handles: number };
  uptime: number;
}

export type HistoryRow = { app: string; kind: string; icon: string | null; cpuS: number; diskB: number; gpuS: number; memPeak: number; last: number };
export type NetApp = { key: string; label: string; icon: string | null; section: Section; recv: number; send: number; conns: number; remotes: string[] };
export type StartupItem = { id: string; name: string; icon: string | null; exec: string; comment: string; source: string; path: string; enabled: boolean };
export type Service = { unit: string; description: string; scope: "system" | "user"; load: string; active: string; sub: string; state: string };

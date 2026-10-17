// Mock data for developing the secondary pages in the browser.
import type { HistoryRow, NetApp, SensorDevice, SensorReading, Service, StartupItem } from "./types";
import { t } from "./i18n.svelte";

const now = Date.now() / 1000;
export const mockHistory = (): HistoryRow[] => [
  { app: "Google Chrome", kind: "app", icon: "/mock-icons/chrome.png", cpuS: 8420, diskB: 3.1e9, gpuS: 2210, memPeak: 3.2e9, last: now - 20 },
  { app: "Claude", kind: "app", icon: "/mock-icons/claude.png", cpuS: 5130, diskB: 9.4e9, gpuS: 410, memPeak: 2.6e9, last: now - 5 },
  { app: "postgres-dev  ·  postgres:16", kind: "ctr", icon: "glyph:container", cpuS: 41200, diskB: 2.2e11, gpuS: 0, memPeak: 7.9e9, last: now - 2 },
  { app: "OpenCode", kind: "app", icon: "/mock-icons/opencode.png", cpuS: 1290, diskB: 2.1e8, gpuS: 380, memPeak: 4.1e8, last: now - 600 },
  { app: "Teams for Linux", kind: "app", icon: "/mock-icons/teams.svg", cpuS: 980, diskB: 1.4e8, gpuS: 120, memPeak: 6.2e8, last: now - 3600 * 3 },
  { app: "Dolphin", kind: "app", icon: "/mock-icons/dolphin.svg", cpuS: 64, diskB: 8.8e7, gpuS: 2, memPeak: 1.2e8, last: now - 86400 },
];
export const mockNet = (): NetApp[] => [
  { key: "a", label: "Google Chrome", icon: "/mock-icons/chrome.png", section: "app", recv: 1.62e6, send: 9.1e4, conns: 38, remotes: ["142.250.180.14:443", "151.101.1.140:443", "[2a00:1450:4002::5e]:443"] },
  { key: "b", label: "Claude", icon: "/mock-icons/claude.png", section: "app", recv: 3.1e5, send: 4.2e4, conns: 12, remotes: ["160.79.104.10:443", "34.36.57.103:443"] },
  { key: "c", label: "postgres-dev", icon: "glyph:container", section: "ctr", recv: 8.4e4, send: 1.2e4, conns: 9, remotes: ["142.250.184.42:443"] },
  { key: "d", label: "Teams for Linux", icon: "/mock-icons/teams.svg", section: "app", recv: 2.2e3, send: 1.1e3, conns: 4, remotes: ["52.113.194.132:443"] },
  { key: "e", label: "Tailscale node agent", icon: null, section: "sys", recv: 640, send: 320, conns: 3, remotes: ["100.64.0.4:41641"] },
];
export const mockStartup = (): StartupItem[] => [
  { id: "org.kde.discover.notifier.desktop", name: t("mock.apps.discoverNotifier"), icon: null, exec: "/usr/lib/DiscoverNotifier", comment: t("mock.startup.discoverComment"), source: "system", path: "/etc/xdg/autostart/org.kde.discover.notifier.desktop", enabled: true },
  { id: "teams-for-linux.desktop", name: "Teams for Linux", icon: "/mock-icons/teams.svg", exec: "teams-for-linux --minimized", comment: t("mock.startup.teamsComment"), source: "user", path: "~/.config/autostart/teams-for-linux.desktop", enabled: true },
  { id: "org.remmina.Remmina-applet.desktop", name: t("mock.apps.remminaApplet"), icon: null, exec: "remmina --icon", comment: t("mock.startup.remminaComment"), source: "system", path: "/etc/xdg/autostart/remmina-applet.desktop", enabled: false },
];
export const mockServices = (): Service[] => [
  { unit: "docker.service", description: "Docker Application Container Engine", scope: "system", load: "loaded", active: "active", sub: "running", state: "enabled" },
  { unit: "NetworkManager.service", description: "Network Manager", scope: "system", load: "loaded", active: "active", sub: "running", state: "enabled" },
  { unit: "bluetooth.service", description: "Bluetooth service", scope: "system", load: "loaded", active: "inactive", sub: "dead", state: "disabled" },
  { unit: "cups.service", description: "CUPS Scheduler", scope: "system", load: "loaded", active: "failed", sub: "failed", state: "enabled" },
  { unit: "pipewire.service", description: "PipeWire Multimedia Service", scope: "user", load: "loaded", active: "active", sub: "running", state: "indirect" },
  { unit: "plasma-plasmashell.service", description: "KDE Plasma Workspace", scope: "user", load: "loaded", active: "active", sub: "running", state: "static" },
];

// sensors: slowly drifting values with statistics since the page opened
const sensorStats = new Map<string, { min: number; max: number; sum: number; n: number }>();
function rd(id: string, label: string, kind: SensorReading["kind"], base: number, amp: number, sub: string | null = null, crit: number | null = null): SensorReading {
  const v = Math.max(0, base + (Math.sin(Date.now() / 3100 + id.length) + Math.random() * 0.6 - 0.3) * amp);
  const s = sensorStats.get(id) ?? { min: v, max: v, sum: 0, n: 0 };
  s.min = Math.min(s.min, v);
  s.max = Math.max(s.max, v);
  s.sum += v;
  s.n++;
  sensorStats.set(id, s);
  const temp = kind === "temp";
  return { id, label, sub, kind, value: v, min: s.min, max: s.max, avg: s.sum / s.n, warn: temp ? (crit ?? 95) - 15 : null, crit: temp ? (crit ?? 93) - 3 : null };
}
export const mockSensors = (): SensorDevice[] => {
  const p = [8, 12, 16, 20, 24, 28].map((c, i) => rd(`cpu:p${i}`, `P-core ${i}`, "temp", 74 + i * 2, 6, `CPU ${i * 2}, ${i * 2 + 1}`, 110));
  const e = Array.from({ length: 8 }, (_, i) => rd(`cpu:e${i}`, `E-core ${i}`, "temp", 66 + (i % 4), 4, `CPU ${12 + i}`, 110));
  const clk = [4300, 2700, 4290, 1950, 2900, 4300].map((f, i) => rd(`cpu:clk${i}`, `P-core ${i}`, "clock", f, 500, `CPU ${i * 2}, ${i * 2 + 1}`));
  return [
    { id: "cpu", name: "Intel Core Ultra 7 155H", kind: "cpu", note: null, readings: [rd("cpu:pkg", "CPU Package", "temp", 88, 6, null, 110), ...p, ...e, ...clk, rd("cpu:usage", "Usage", "percent", 34, 10)] },
    { id: "gpu0", name: "Intel Arc Graphics", kind: "gpu", note: null, readings: [rd("gpu0:util", "Usage", "percent", 14, 6), rd("gpu0:mem", "Memory used", "memory", 1.9e9, 1e8)] },
    { id: "gpu1", name: "NVIDIA RTX 500 Ada Generation Laptop GPU", kind: "gpu", note: "Suspended: it is not woken up just to read its sensors", readings: [] },
    { id: "memory", name: "DDR5 memory", kind: "memory", note: null, readings: [rd("mem:1", "Module 1", "temp", 59, 1.5, "address 0x50", 85), rd("mem:2", "Module 2", "temp", 59.5, 1.5, "address 0x55", 85)] },
    { id: "nvme0", name: "SK hynix HFS001TEJ9X162N", kind: "disk", note: null, readings: [rd("nvme:c", "Composite", "temp", 49, 2, null, 86.85), rd("nvme:1", "Sensor 1", "temp", 52, 2), rd("nvme:2", "Sensor 2", "temp", 43, 1)] },
    { id: "wifi", name: "Intel Meteor Lake PCH CNVi WiFi", kind: "network", note: null, readings: [rd("wifi:t", "Temperature", "temp", 45, 1)] },
    { id: "ec", name: "ThinkPad embedded controller", kind: "board", note: null, readings: [rd("ec:cpu", "CPU", "temp", 90, 6), rd("ec:gpu", "GPU", "temp", 51, 2), rd("ec:fan", "Fan 1", "fan", 4700, 400)] },
    { id: "bat0", name: "Battery Celxpert 5B11M90093", kind: "battery", note: "Charged", readings: [rd("bat:c", "Charge level", "percent", 100, 0), rd("bat:v", "Voltage", "volt", 17.23, 0.01), rd("bat:w", "Wear level", "percent", 41.1, 0), rd("bat:n", "Charge cycles", "count", 132, 0)] },
  ];
};

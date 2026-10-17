// Dati simulati per sviluppare le pagine secondarie nel browser.
import type { HistoryRow, NetApp, Service, StartupItem } from "./types";

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
  { id: "org.kde.discover.notifier.desktop", name: "Notifiche di Discover", icon: null, exec: "/usr/lib/DiscoverNotifier", comment: "Aggiornamenti disponibili", source: "system", path: "/etc/xdg/autostart/org.kde.discover.notifier.desktop", enabled: true },
  { id: "teams-for-linux.desktop", name: "Teams for Linux", icon: "/mock-icons/teams.svg", exec: "teams-for-linux --minimized", comment: "Client non ufficiale di Microsoft Teams", source: "user", path: "~/.config/autostart/teams-for-linux.desktop", enabled: true },
  { id: "org.remmina.Remmina-applet.desktop", name: "Applet di Remmina", icon: null, exec: "remmina --icon", comment: "Connessioni desktop remoto", source: "system", path: "/etc/xdg/autostart/remmina-applet.desktop", enabled: false },
];
export const mockServices = (): Service[] => [
  { unit: "docker.service", description: "Docker Application Container Engine", scope: "system", load: "loaded", active: "active", sub: "running", state: "enabled" },
  { unit: "NetworkManager.service", description: "Network Manager", scope: "system", load: "loaded", active: "active", sub: "running", state: "enabled" },
  { unit: "bluetooth.service", description: "Bluetooth service", scope: "system", load: "loaded", active: "inactive", sub: "dead", state: "disabled" },
  { unit: "cups.service", description: "CUPS Scheduler", scope: "system", load: "loaded", active: "failed", sub: "failed", state: "enabled" },
  { unit: "pipewire.service", description: "PipeWire Multimedia Service", scope: "user", load: "loaded", active: "active", sub: "running", state: "indirect" },
  { unit: "plasma-plasmashell.service", description: "KDE Plasma Workspace", scope: "user", load: "loaded", active: "active", sub: "running", state: "static" },
];

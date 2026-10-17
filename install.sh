#!/usr/bin/env bash
# Installs Gestione attività for the current user (KDE Plasma 6, Wayland):
#   Ctrl+Alt+Del   → Windows-style security screen (Lock, Switch user, Sign out, Task Manager)
#   Ctrl+Shift+Esc → Gestione attività
# History and settings are kept. A previous Python version, if any, is moved to the trash.
set -euo pipefail

SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
CONF="${XDG_CONFIG_HOME:-$HOME/.config}"
DEST="$DATA/gestione-attivita"
BIN="$HOME/.local/bin"
APPS="$DATA/applications"
ICONS="$DATA/icons/hicolor"
# Two ways to use it: from the release package (binary next to this script) or from the sources.
if [ -x "$SRC/gestione-attivita" ] && [ ! -d "$SRC/src-tauri" ]; then
    EXE="$SRC/gestione-attivita"
    ICON_SVG="$SRC/icon.svg"
else
    EXE="$SRC/src-tauri/target/release/gestione-attivita"
    ICON_SVG="$SRC/src-tauri/icons/icon.svg"
    if [ ! -x "$EXE" ]; then
        echo "→ Building the app (the first build takes a few minutes)…"
        (cd "$SRC" && npm install --silent && npx tauri build)
    fi
fi

echo "→ Closing running instances…"
pkill -f "gestione_attivita.py" 2>/dev/null || true
pkill -f "$DEST/bin/gestione-attivita" 2>/dev/null || true
sleep 0.5

echo "→ Copying the app to $DEST/bin…"
mkdir -p "$DEST/bin" "$BIN" "$APPS" "$ICONS/scalable/apps"
install -m 755 "$EXE" "$DEST/bin/gestione-attivita.new"
mv -f "$DEST/bin/gestione-attivita.new" "$DEST/bin/gestione-attivita"
ln -sfn "$DEST/bin/gestione-attivita" "$BIN/gestione-attivita"

echo "→ Icon…"
cp "$ICON_SVG" "$ICONS/scalable/apps/gestione-attivita.svg"
for s in 32 48 64 128 256; do
    mkdir -p "$ICONS/${s}x${s}/apps"
    if command -v rsvg-convert >/dev/null; then
        rsvg-convert -w $s -h $s "$ICON_SVG" -o "$ICONS/${s}x${s}/apps/gestione-attivita.png"
    fi
done
touch "$ICONS" 2>/dev/null || true

echo "→ Menu entries…"
cat > "$APPS/gestione-attivita.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Gestione attività
GenericName=Task Manager
Comment=Processes, performance, app history, network, startup apps, users and services
Comment[it]=Processi, prestazioni, cronologia, rete, app di avvio, utenti e servizi
Exec=$BIN/gestione-attivita
Icon=gestione-attivita
Terminal=false
Categories=System;Monitor;
Keywords=task;manager;taskmgr;processi;prestazioni;servizi;rete;
StartupNotify=true
StartupWMClass=gestione-attivita
X-KDE-Shortcuts=Ctrl+Shift+Esc
DESKTOP
cat > "$APPS/gestione-attivita-sicurezza.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Ctrl+Alt+Del screen
Name[it]=Schermata Ctrl+Alt+Canc
Comment=Lock, switch user, sign out or open the task manager
Comment[it]=Blocca, cambia utente, esci o apri Gestione attività
Exec=$BIN/gestione-attivita --secure
Icon=gestione-attivita
Terminal=false
NoDisplay=true
StartupNotify=false
X-KDE-Shortcuts=Ctrl+Alt+Del
DESKTOP
update-desktop-database "$APPS" >/dev/null 2>&1 || true
kbuildsycoca6 >/dev/null 2>&1 || true

echo "→ Start at login, in the system tray (so Ctrl+Alt+Del responds instantly)…"
mkdir -p "$CONF/autostart" "$CONF/gestione-attivita"
cat > "$CONF/autostart/gestione-attivita.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Gestione attività
Comment=Stays in the system tray, ready for Ctrl+Alt+Del
Comment[it]=Resta nella barra di sistema, pronta per Ctrl+Alt+Canc
Exec=$BIN/gestione-attivita --tray
Icon=gestione-attivita
X-GNOME-Autostart-enabled=true
NoDisplay=true
DESKTOP
SETTINGS="$CONF/gestione-attivita/impostazioni.json"
if [ -f "$SETTINGS" ] && command -v python3 >/dev/null; then
    python3 - "$SETTINGS" <<'PY'
import json, sys
p = sys.argv[1]
s = json.load(open(p))
s.update(autostart=True, tray=True, shortcuts=True)
json.dump(s, open(p, "w"), indent=2)
PY
elif [ ! -f "$SETTINGS" ]; then
    printf '{\n  "theme": "graphite",\n  "intervalMs": 1000,\n  "alerts": true,\n  "tray": true,\n  "autostart": true,\n  "shortcuts": true\n}\n' > "$SETTINGS"
fi

echo "→ Keyboard shortcuts…"
SHORTCUTS="$CONF/kglobalshortcutsrc"
[ -f "$SHORTCUTS" ] && [ ! -f "$SHORTCUTS.bak-gestione-attivita" ] && cp "$SHORTCUTS" "$SHORTCUTS.bak-gestione-attivita"
if "$DEST/bin/gestione-attivita" --apply-shortcuts; then
    echo "  Ctrl+Alt+Del opens the security screen, Ctrl+Shift+Esc opens Gestione attività"
else
    echo "  KDE did not respond: assign the shortcuts in System Settings → Shortcuts."
fi

echo "→ Checking for previous versions…"
old=()
for p in "$DEST/app" "$DEST/venv" "$CONF/gestione-attivita/gestione-attivita.conf"; do
    [ -e "$p" ] && old+=("$p")
done
if [ ${#old[@]} -gt 0 ]; then
    if command -v gio >/dev/null && gio trash "${old[@]}" 2>/dev/null; then
        echo "  Moved to the trash: ${old[*]}"
    else
        mkdir -p "$DEST/old-python-version"
        mv "${old[@]}" "$DEST/old-python-version/"
        echo "  Moved to $DEST/old-python-version"
    fi
fi

echo "→ Starting in the system tray…"
setsid "$DEST/bin/gestione-attivita" --tray >/dev/null 2>&1 < /dev/null &

echo
echo "Installed. Press Ctrl+Alt+Del or Ctrl+Shift+Esc."

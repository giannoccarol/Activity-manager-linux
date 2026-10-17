#!/usr/bin/env bash
# Rimuove Gestione attività e riporta Ctrl+Alt+Canc a «Esci» di KDE.
# Cronologia e impostazioni restano in ~/.local/share/gestione-attivita e ~/.config/gestione-attivita.
set -euo pipefail

DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
CONF="${XDG_CONFIG_HOME:-$HOME/.config}"
DEST="$DATA/gestione-attivita"

[ -x "$DEST/bin/gestione-attivita" ] && "$DEST/bin/gestione-attivita" --remove-shortcuts || true
pkill -f "$DEST/bin/gestione-attivita" 2>/dev/null || true
rm -f "$HOME/.local/bin/gestione-attivita" "$DEST/bin/gestione-attivita"
rmdir "$DEST/bin" 2>/dev/null || true
rm -f "$DATA/applications/gestione-attivita.desktop" "$DATA/applications/gestione-attivita-sicurezza.desktop"
rm -f "$CONF/autostart/gestione-attivita.desktop"
find "$DATA/icons/hicolor" -name "gestione-attivita.*" -delete 2>/dev/null || true
kbuildsycoca6 >/dev/null 2>&1 || true
echo "Gestione attività rimossa. Ctrl+Alt+Canc torna a «Esci»."

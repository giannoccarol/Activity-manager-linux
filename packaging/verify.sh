#!/usr/bin/env bash
# Checks a release package the way the in-app updater (src-tauri/src/update.rs) will use it:
# asset names, the "<sha256>  <file>" checksum file, the folder layout, install.sh with update mode,
# and a binary that starts and prints exactly the version.
#
#   packaging/verify.sh gestione-attivita-X.Y.Z-linux-x86_64.tar.gz [X.Y.Z]
#
# The .sha256 file must sit next to the tarball. Without a version, it is read from the file name.
set -euo pipefail

TARBALL="$1"
FILE="$(basename "$TARBALL")"
VERSION="${2:-$(echo "$FILE" | sed -n 's/^gestione-attivita-\(.*\)-linux-x86_64\.tar\.gz$/\1/p')}"
NAME="gestione-attivita-$VERSION-linux-x86_64"
fail() { echo "::error::$FILE: $*" >&2; exit 1; }

[ "$FILE" = "$NAME.tar.gz" ] || fail "the updater looks for $NAME.tar.gz"
[ -f "$TARBALL.sha256" ] || fail "missing $FILE.sha256"

# checksum: one line, "<64 lowercase hex>  <file name>"
[ "$(wc -l < "$TARBALL.sha256")" = 1 ] || fail "$FILE.sha256 must be a single line"
grep -Eqx "[0-9a-f]{64}  $(printf '%s' "$FILE" | sed 's/\./\\./g')" "$TARBALL.sha256" || fail "$FILE.sha256 is not \"<sha256>  $FILE\""
[ "$(cut -d' ' -f1 "$TARBALL.sha256")" = "$(sha256sum "$TARBALL" | cut -d' ' -f1)" ] || fail "checksum does not match"

# layout: one folder with exactly these files
expected="$(printf '%s\n' "$NAME/" "$NAME/LICENSE" "$NAME/README.txt" "$NAME/gestione-attivita" "$NAME/icon.svg" "$NAME/install.sh" "$NAME/uninstall.sh")"
actual="$(tar -tzf "$TARBALL" | sed 's|^\./||' | LC_ALL=C sort)"
[ "$actual" = "$(echo "$expected" | LC_ALL=C sort)" ] || fail "unexpected contents:
$actual"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
tar -xzf "$TARBALL" -C "$TMP"
PKG="$TMP/$NAME"
for f in gestione-attivita install.sh uninstall.sh; do
    [ -f "$PKG/$f" ] && [ -x "$PKG/$f" ] || fail "$f is not an executable file"
done
for f in LICENSE README.txt icon.svg; do
    [ -f "$PKG/$f" ] && [ -s "$PKG/$f" ] || fail "$f is missing or empty"
done
grep -q 'GA_UPDATE' "$PKG/install.sh" || fail "install.sh has no update mode (GA_UPDATE)"
head -n 1 "$PKG/README.txt" | grep -qF "Activity Manager $VERSION " || fail "README.txt does not name version $VERSION"
grep -q '<svg' "$PKG/icon.svg" || fail "icon.svg is not an SVG image"

# the updater runs `<new binary> --version` and expects exactly the version
out="$(cd "$TMP" && env -u GA_UPDATE_TEST_VERSION -u XDG_ACTIVATION_TOKEN -u DESKTOP_STARTUP_ID timeout 10 "$PKG/gestione-attivita" --version < /dev/null 2> /dev/null)" || fail "the binary does not start (--version)"
[ "$out" = "$VERSION" ] || fail "--version prints \"$out\", expected \"$VERSION\""

echo "$FILE: OK (checksum, layout, install.sh update mode, --version $VERSION)"

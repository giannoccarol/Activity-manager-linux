#!/usr/bin/env bash
# Builds the release package from the binary made by `npx tauri build`:
#   src-tauri/target/dist/gestione-attivita-<version>-linux-x86_64.tar.gz (+ .sha256)
# then checks it the way the in-app updater does (packaging/verify.sh).
#
#   packaging/package.sh [vX.Y.Z]          the tag, if given, must match the version
#   packaging/package.sh --check [vX.Y.Z]  only check the version (before building)
#
# The version comes from src-tauri/Cargo.toml; package.json and tauri.conf.json must agree.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHECK_ONLY=0
if [ "${1:-}" = "--check" ]; then CHECK_ONLY=1; shift; fi
TAG="${1:-}"

VERSION="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$ROOT/src-tauri/Cargo.toml" | head -n 1)"
NPM_VERSION="$(sed -n 's/^  "version": "\(.*\)",$/\1/p' "$ROOT/package.json" | head -n 1)"
TAURI_VERSION="$(sed -n 's/^  "version": "\(.*\)",$/\1/p' "$ROOT/src-tauri/tauri.conf.json" | head -n 1)"
fail() { echo "::error::$*" >&2; exit 1; }
[ -n "$VERSION" ] || fail "no version in src-tauri/Cargo.toml"
[ "$NPM_VERSION" = "$VERSION" ] || fail "package.json says ${NPM_VERSION:-nothing}, src-tauri/Cargo.toml says $VERSION"
[ "$TAURI_VERSION" = "$VERSION" ] || fail "src-tauri/tauri.conf.json says ${TAURI_VERSION:-nothing}, src-tauri/Cargo.toml says $VERSION"
[ -z "$TAG" ] || [ "$TAG" = "v$VERSION" ] || fail "tag $TAG does not match version $VERSION in src-tauri/Cargo.toml (expected v$VERSION)"
echo "Version $VERSION${TAG:+ (tag $TAG)}"
[ -z "${GITHUB_OUTPUT:-}" ] || echo "version=$VERSION" >> "$GITHUB_OUTPUT"
[ "$CHECK_ONLY" = 0 ] || exit 0

NAME="gestione-attivita-$VERSION-linux-x86_64"
EXE="$ROOT/src-tauri/target/release/gestione-attivita"
OUT="$ROOT/src-tauri/target/dist"
[ -x "$EXE" ] || fail "$EXE not found: run npx tauri build first"

rm -rf "$OUT"
mkdir -p "$OUT/$NAME"
install -m 755 "$EXE" "$OUT/$NAME/gestione-attivita"
install -m 755 "$ROOT/install.sh" "$ROOT/uninstall.sh" "$OUT/$NAME/"
install -m 644 "$ROOT/src-tauri/icons/icon.svg" "$ROOT/LICENSE" "$OUT/$NAME/"
sed "s/@VERSION@/$VERSION/g" "$ROOT/packaging/README.txt" > "$OUT/$NAME/README.txt"
chmod 644 "$OUT/$NAME/README.txt"

# same bytes for the same inputs: fixed owner, order and dates
EPOCH="${SOURCE_DATE_EPOCH:-$(git -C "$ROOT" log -1 --format=%ct 2>/dev/null || date +%s)}"
(cd "$OUT" && tar --sort=name --owner=0 --group=0 --numeric-owner --mtime="@$EPOCH" -cf - "$NAME" | gzip -9n > "$NAME.tar.gz")
(cd "$OUT" && sha256sum "$NAME.tar.gz" > "$NAME.tar.gz.sha256")
rm -rf "${OUT:?}/$NAME"

"$ROOT/packaging/verify.sh" "$OUT/$NAME.tar.gz" "$VERSION"

SUM="$(cut -d' ' -f1 "$OUT/$NAME.tar.gz.sha256")"
echo "$OUT/$NAME.tar.gz"
echo "sha256 $SUM"
if [ -n "${GITHUB_OUTPUT:-}" ]; then
    echo "tarball=$OUT/$NAME.tar.gz" >> "$GITHUB_OUTPUT"
    echo "sha256=$SUM" >> "$GITHUB_OUTPUT"
fi
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    {
        echo "### $NAME.tar.gz"
        echo
        echo "SHA-256 \`$SUM\`"
        echo
        echo "AUR (packaging/aur/gestione-attivita-bin/PKGBUILD): \`pkgver=$VERSION\`, \`pkgrel=1\`, \`sha256sums=('$SUM')\`"
    } >> "$GITHUB_STEP_SUMMARY"
fi

#!/usr/bin/env bash
set -euo pipefail

REPO_URL="https://github.com/IvanLi-CN/isolarail"
VERSION="latest"
INSTALL_DIR="${HOME}/.local/bin"
FORCE=0
DRY_RUN=0

usage() {
  cat <<'EOF'
Install IsolaRail companion tools for the current user.

Usage:
  install-isolarail-companion.sh [--version <tag>] [--install-dir <dir>] [--force] [--dry-run]

Defaults:
  --version latest
  --install-dir ~/.local/bin
EOF
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      [ "$#" -ge 2 ] || die "--version requires a value"
      VERSION="$2"
      shift 2
      ;;
    --install-dir)
      [ "$#" -ge 2 ] || die "--install-dir requires a value"
      INSTALL_DIR="$2"
      shift 2
      ;;
    --force)
      FORCE=1
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

case "$(uname -s)" in
  Darwin)
    [ "$(uname -m)" = "arm64" ] || die "unsupported macOS architecture: $(uname -m); expected arm64"
    SLUG="macos-aarch64"
    ;;
  Linux)
    [ "$(uname -m)" = "x86_64" ] || die "unsupported Linux architecture: $(uname -m); expected x86_64"
    SLUG="linux-x86_64"
    ;;
  *)
    die "unsupported operating system: $(uname -s)"
    ;;
esac

ARCHIVE="isolarail-companion-tools-${SLUG}.tar.gz"
if [ "$VERSION" = "latest" ]; then
  BASE_URL="${REPO_URL}/releases/latest/download"
else
  BASE_URL="${REPO_URL}/releases/download/${VERSION}"
fi
ARCHIVE_URL="${BASE_URL}/${ARCHIVE}"
CHECKSUM_URL="${BASE_URL}/SHA256SUMS"

printf 'IsolaRail companion tools install plan\n'
printf '  source: %s\n' "$BASE_URL"
printf '  archive: %s\n' "$ARCHIVE"
printf '  install_dir: %s\n' "$INSTALL_DIR"
printf '  force: %s\n' "$FORCE"

if [ "$DRY_RUN" -eq 1 ]; then
  printf 'dry-run: no files downloaded or installed\n'
  exit 0
fi

need_cmd tar
need_cmd curl

if command -v shasum >/dev/null 2>&1; then
  SHA_CMD="shasum -a 256"
elif command -v sha256sum >/dev/null 2>&1; then
  SHA_CMD="sha256sum"
else
  die "missing checksum command: shasum or sha256sum"
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

archive_path="${tmp_dir}/${ARCHIVE}"
checksums_path="${tmp_dir}/SHA256SUMS"
archive_effective="${tmp_dir}/archive.effective-url"

curl -fsSL -w '%{url_effective}' -o "$archive_path" "$ARCHIVE_URL" > "$archive_effective"
curl -fsSL -o "$checksums_path" "$CHECKSUM_URL"

target_tag="$VERSION"
if [ "$VERSION" = "latest" ]; then
  target_tag="$(sed -n 's#.*\/releases\/download\/\([^/]*\)\/.*#\1#p' "$archive_effective")"
  [ -n "$target_tag" ] || target_tag="latest"
fi

normalize_version() {
  printf '%s' "$1" | sed -E 's/^[^0-9]*//; s/[-+].*$//'
}

semver_cmp() {
  awk -v a="$(normalize_version "$1")" -v b="$(normalize_version "$2")" '
    BEGIN {
      na = split(a, av, "."); nb = split(b, bv, ".");
      for (i = 1; i <= 3; i++) {
        ai = (i <= na && av[i] ~ /^[0-9]+$/) ? av[i] + 0 : 0;
        bi = (i <= nb && bv[i] ~ /^[0-9]+$/) ? bv[i] + 0 : 0;
        if (ai < bi) { print -1; exit }
        if (ai > bi) { print 1; exit }
      }
      print 0
    }'
}

installed_version=""
installed_isolarail=""
if [ -x "${INSTALL_DIR}/isolarail" ]; then
  installed_isolarail="${INSTALL_DIR}/isolarail"
elif command -v isolarail >/dev/null 2>&1; then
  installed_isolarail="$(command -v isolarail)"
fi
if [ -n "$installed_isolarail" ]; then
  installed_version="$("$installed_isolarail" --version 2>/dev/null | awk '{print $NF}' || true)"
fi
devd_available=0
installed_devd=""
if [ -x "${INSTALL_DIR}/isolarail-devd" ]; then
  installed_devd="${INSTALL_DIR}/isolarail-devd"
elif command -v isolarail-devd >/dev/null 2>&1; then
  installed_devd="$(command -v isolarail-devd)"
fi
if [ -n "$installed_devd" ] && "$installed_devd" --help >/dev/null 2>&1; then
  devd_available=1
fi

target_version="$(normalize_version "$target_tag")"
if [ -n "$installed_version" ] && [ -n "$target_version" ]; then
  installed_norm="$(normalize_version "$installed_version")"
  cmp="$(semver_cmp "$target_version" "$installed_norm")"
  if [ "$cmp" -eq 0 ] && [ "$FORCE" -ne 1 ] && [ "$devd_available" -eq 1 ]; then
    printf 'isolarail %s is already installed; use --force to reinstall\n' "$installed_version"
    exit 0
  fi
  if [ "$cmp" -lt 0 ] && [ "$FORCE" -ne 1 ]; then
    die "refusing to downgrade isolarail ${installed_version} to ${target_tag}; use --force to override"
  fi
fi

expected_sha="$(awk -v file="$ARCHIVE" '$2 == file { print $1 }' "$checksums_path")"
[ -n "$expected_sha" ] || die "SHA256SUMS does not contain ${ARCHIVE}"
actual_sha="$($SHA_CMD "$archive_path" | awk '{ print $1 }')"
[ "$expected_sha" = "$actual_sha" ] || die "checksum mismatch for ${ARCHIVE}"

extract_dir="${tmp_dir}/extract"
mkdir -p "$extract_dir"
tar -xzf "$archive_path" -C "$extract_dir"

[ -f "${extract_dir}/isolarail" ] || die "archive missing isolarail"
[ -f "${extract_dir}/isolarail-devd" ] || die "archive missing isolarail-devd"

mkdir -p "$INSTALL_DIR"
install -m 0755 "${extract_dir}/isolarail" "${INSTALL_DIR}/isolarail"
install -m 0755 "${extract_dir}/isolarail-devd" "${INSTALL_DIR}/isolarail-devd"

"${INSTALL_DIR}/isolarail" --help >/dev/null
"${INSTALL_DIR}/isolarail-devd" --help >/dev/null

printf 'installed IsolaRail companion tools to %s\n' "$INSTALL_DIR"
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    printf 'PATH note: add this directory before using isolarail from a new shell:\n'
    printf '  export PATH="%s:$PATH"\n' "$INSTALL_DIR"
    ;;
esac

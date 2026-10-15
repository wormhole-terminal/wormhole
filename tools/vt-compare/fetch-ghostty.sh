#!/usr/bin/env bash
# Build libghostty-vt (Ghostty's terminal engine as a C library) at the commit in ghostty.pin, into
# a cache outside the repository, so vt-compare's build.rs can link it.
#
#   tools/vt-compare/fetch-ghostty.sh            # clone, get the right Zig, build (a few minutes)
#   tools/vt-compare/fetch-ghostty.sh --force    # build again even when the library is there
#   tools/vt-compare/fetch-ghostty.sh --path     # print the folder with lib/ and include/, build nothing
#
# Everything lands in $WORMHOLE_VT_COMPARE_CACHE (default ~/Library/Caches/wormhole/vt-compare on
# macOS, ${XDG_CACHE_HOME:-~/.cache}/wormhole/vt-compare elsewhere):
#   ghostty-src/               the checkout at the pinned commit
#   zig-<version>/             the Zig compiler, when the installed one is not the version Ghostty asks for
#   zig-cache/                 Zig's package and build cache (ZIG_GLOBAL_CACHE_DIR), kept here
#   ghostty-vt/<rev>/          lib/libghostty-vt.a and include/ghostty/, what build.rs reads
#
# Only the build runs here; nothing of Ghostty's source is read by this script except the Zig
# version its build.zig.zon names.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
pin() { sed -n "s/^$1=//p" "$here/ghostty.pin" | head -n1; }
rev="$(pin rev)"
zig_version="$(pin zig)"
[ -n "$rev" ] && [ -n "$zig_version" ] || { echo "error: $here/ghostty.pin has no rev or zig" >&2; exit 1; }

if [ -n "${WORMHOLE_VT_COMPARE_CACHE:-}" ]; then
  cache="$WORMHOLE_VT_COMPARE_CACHE"
elif [ "$(uname -s)" = Darwin ]; then
  cache="$HOME/Library/Caches/wormhole/vt-compare"
else
  cache="${XDG_CACHE_HOME:-$HOME/.cache}/wormhole/vt-compare"
fi
out="$cache/ghostty-vt/${rev:0:12}"

force=0
for arg in "$@"; do
  case "$arg" in
    --force) force=1 ;;
    --path) echo "$out"; exit 0 ;;
    -h|--help) sed -n '2,17p' "$0"; exit 0 ;;
    *) echo "error: unknown argument $arg (see --help)" >&2; exit 1 ;;
  esac
done

log() { printf '\033[1;34m==>\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

if [ "$force" = 0 ] && [ -f "$out/lib/libghostty-vt.a" ] && [ -f "$out/include/ghostty/vt.h" ]; then
  log "libghostty-vt ${rev:0:12} is already built: $out"
  exit 0
fi
command -v git >/dev/null || die "git is needed"
mkdir -p "$cache"

# 1. The source at exactly the pinned commit (a shallow fetch by hash: GitHub serves any commit).
src="$cache/ghostty-src"
if [ ! -d "$src/.git" ]; then
  git init -q "$src"
  git -C "$src" remote add origin https://github.com/ghostty-org/ghostty.git
fi
if [ "$(git -C "$src" rev-parse -q --verify HEAD 2>/dev/null || true)" != "$rev" ]; then
  log "fetching ghostty ${rev:0:12}"
  git -C "$src" fetch -q --depth 1 origin "$rev"
  git -C "$src" checkout -q --detach --force "$rev"
fi
[ "$(git -C "$src" rev-parse HEAD)" = "$rev" ] || die "the checkout is not at $rev"

# 2. The Zig it asks for. Ghostty refuses any other minor version, so a newer system Zig is no use.
wanted="$(sed -n 's/.*minimum_zig_version = "\([0-9.]*\)".*/\1/p' "$src/build.zig.zon" | head -n1)"
[ "$wanted" = "$zig_version" ] || die "ghostty.pin says zig $zig_version but that commit asks for $wanted: update ghostty.pin"
zig=""
if command -v zig >/dev/null && [ "$(zig version)" = "$zig_version" ]; then
  zig="$(command -v zig)"
else
  case "$(uname -s)-$(uname -m)" in
    Darwin-arm64)  platform=aarch64-macos ;;
    Darwin-x86_64) platform=x86_64-macos ;;
    Linux-aarch64|Linux-arm64) platform=aarch64-linux ;;
    Linux-x86_64)  platform=x86_64-linux ;;
    *) die "no Zig $zig_version download known for $(uname -s)-$(uname -m)" ;;
  esac
  # sha256 of the tarballs on ziglang.org/download/index.json
  case "$zig_version-$platform" in
    0.16.0-aarch64-macos) sha=b23d70deaa879b5c2d486ed3316f7eaa53e84acf6fc9cc747de152450d401489 ;;
    0.16.0-x86_64-macos)  sha=0387557ed1877bc6a2e1802c8391953baddba76081876301c522f52977b52ba7 ;;
    0.16.0-x86_64-linux)  sha=70e49664a74374b48b51e6f3fdfbf437f6395d42509050588bd49abe52ba3d00 ;;
    0.16.0-aarch64-linux) sha=ea4b09bfb22ec6f6c6ceac57ab63efb6b46e17ab08d21f69f3a48b38e1534f17 ;;
    *) die "no checksum for zig $zig_version on $platform: add it to this script" ;;
  esac
  zig_dir="$cache/zig-$zig_version"
  if [ ! -x "$zig_dir/zig" ] || [ "$("$zig_dir/zig" version)" != "$zig_version" ]; then
    log "downloading zig $zig_version ($platform)"
    tmp="$(mktemp -d "$cache/zig-dl.XXXXXX")"
    archive="zig-$platform-$zig_version.tar.xz"
    curl -fsSL "https://ziglang.org/download/$zig_version/$archive" -o "$tmp/$archive"
    echo "$sha  $tmp/$archive" | shasum -a 256 -c - >/dev/null || die "checksum of $archive differs"
    mkdir -p "$zig_dir"
    tar -xJf "$tmp/$archive" -C "$zig_dir" --strip-components=1
    rm -r "$tmp"
  fi
  zig="$zig_dir/zig"
fi
log "zig: $zig ($("$zig" version))"

# 3. The library alone (-Demit-lib-vt leaves out the macOS app and the docs), optimized as a
# program embedding it would build it.
log "building libghostty-vt (ReleaseFast)"
stage="$(mktemp -d "$cache/ghostty-build.XXXXXX")"
(cd "$src" && ZIG_GLOBAL_CACHE_DIR="$cache/zig-cache" "$zig" build -Demit-lib-vt -Doptimize=ReleaseFast --prefix "$stage")
[ -f "$stage/lib/libghostty-vt.a" ] || die "the build made no lib/libghostty-vt.a"

# 4. Keep what is linked and the headers it is compiled against; the rest of the prefix is not needed.
rm -rf "$out"
mkdir -p "$out/lib"
cp "$stage/lib/libghostty-vt.a" "$out/lib/"
cp -R "$stage/include" "$out/include"
printf 'rev=%s\nzig=%s\noptimize=ReleaseFast\n' "$rev" "$zig_version" > "$out/BUILD"
rm -r "$stage"
log "done: $out"

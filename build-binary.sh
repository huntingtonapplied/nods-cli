#!/usr/bin/env bash
# Build the nods CLI release binary for the downloads CDN (R-tea-23).
#
# By default rustc embeds the build host's absolute source paths (the crate dir
# under $HOME and the dependency registry under $CARGO_HOME) into the binary, so
# `strings nods | grep /Users/ or /home/` leaks the build account's layout.
# This wrapper remaps those prefixes with --remap-path-prefix (RUSTFLAGS) and
# strips symbols so the shipped binary carries no host paths.
#
# strip + lto are also set in Cargo.toml [profile.release]; RUSTFLAGS cannot be
# expressed in .cargo/config.toml because it needs $HOME / $CARGO_HOME expanded
# at build time, which config.toml does not support. Run this script instead.
#
# Usage: ./build-binary.sh   (run from nods/cli/)
set -euo pipefail

cd "$(dirname "$0")"

os=$(uname -s | tr '[:upper:]' '[:lower:]'); case "$os" in darwin) OS=darwin;; linux) OS=linux;; *) OS=$os;; esac
m=$(uname -m); case "$m" in x86_64|amd64) ARCH=x86_64;; arm64|aarch64) ARCH=arm64;; *) ARCH=$m;; esac
BIN="nods-${OS}-${ARCH}"

CARGO="$HOME/.cargo/bin/cargo"; [ -x "$CARGO" ] || CARGO=cargo
CH="${CARGO_HOME:-$HOME/.cargo}"

export RUSTFLAGS="--remap-path-prefix=$HOME=~ --remap-path-prefix=$CH=/cargo"
"$CARGO" build --release --bin nods

# Locate the artifact (Cargo.toml's .cargo/config.toml redirects target-dir).
OUT=$("$CARGO" metadata --no-deps --format-version 1 2>/dev/null \
      | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')/release/nods
[ -f "$OUT" ] || OUT="../../environments/cli_target/target/release/nods"
[ -f "$OUT" ] || { echo "nods binary not found after build" >&2; exit 1; }

strip "$OUT" 2>/dev/null || true
mkdir -p dist
cp "$OUT" "dist/$BIN"
chmod 755 "dist/$BIN"

LEAK=$(strings "dist/$BIN" | grep -c "/Users/\|/home/" || true)
echo "Built dist/$BIN  (host-path leak count: $LEAK  — must be 0)"
"dist/$BIN" --version
{ sha256sum "dist/$BIN" 2>/dev/null || shasum -a 256 "dist/$BIN"; } > dist/SHA256SUMS

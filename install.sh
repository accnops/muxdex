#!/bin/sh
set -eu

OWNER_REPO="accnops/muxdex"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"

os="$(uname -s)"
arch="$(uname -m)"

if [ "$os" != "Darwin" ]; then
  echo "muxdex currently publishes release binaries for macOS only." >&2
  exit 1
fi

case "$arch" in
  arm64)
    target="aarch64-apple-darwin"
    ;;
  x86_64)
    target="x86_64-apple-darwin"
    ;;
  *)
    echo "Unsupported macOS architecture: $arch" >&2
    exit 1
    ;;
esac

latest_json="$(curl -fsSL "https://api.github.com/repos/$OWNER_REPO/releases/latest")"
tag="$(printf '%s\n' "$latest_json" | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"

if [ -z "$tag" ]; then
  echo "Could not determine the latest muxdex release tag." >&2
  exit 1
fi

asset="muxdex-$target.tar.gz"
url="https://github.com/$OWNER_REPO/releases/download/$tag/$asset"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT INT TERM

mkdir -p "$BIN_DIR"
curl -fsSL "$url" -o "$tmpdir/$asset"
tar -xzf "$tmpdir/$asset" -C "$tmpdir"
install "$tmpdir/muxdex" "$BIN_DIR/muxdex"

echo "Installed muxdex $tag to $BIN_DIR/muxdex"
case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *)
    echo "Note: add $BIN_DIR to your PATH if it is not already there."
    ;;
esac

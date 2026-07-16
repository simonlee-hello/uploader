#!/usr/bin/env bash
# Cross-compile Rust uploader for common targets (best-effort).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/rust"
export PATH="${HOME}/.cargo/bin:${PATH}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}"

OUT="$ROOT/bin"
mkdir -p "$OUT/osx" "$OUT/linux" "$OUT/windows"

need_target() {
  local t="$1"
  rustup target list --installed | grep -qx "$t" || rustup target add "$t"
}

build_one() {
  local triple="$1" dest="$2"
  echo "==> $triple -> $dest"
  if ! need_target "$triple" 2>/dev/null; then
    echo "skip: cannot install $triple"
    return 0
  fi
  if cargo build --release --target "$triple" -q; then
    local src="target/${triple}/release/uploader"
    [[ -f "${src}.exe" ]] && src="${src}.exe"
    if [[ -f "$src" ]]; then
      cp "$src" "$dest"
      ls -lh "$dest"
    fi
  else
    echo "skip: build failed for $triple (missing linker?)"
  fi
}

HOST=$(rustc -vV | sed -n 's/^host: //p')
echo "host: $HOST"

# Native
cargo build --release -q
NATIVE="$OUT/osx/uploader-rust-darwin-$(uname -m)"
if [[ "$(uname -s)" == "Darwin" ]]; then
  cp target/release/uploader "$NATIVE"
  ls -lh "$NATIVE"
elif [[ "$(uname -s)" == "Linux" ]]; then
  cp target/release/uploader "$OUT/linux/uploader-rust-linux-$(uname -m)"
  ls -lh "$OUT/linux/uploader-rust-linux-$(uname -m)"
fi

# Cross (may skip without linker / zig)
case "$(uname -s)" in
  Darwin)
    build_one x86_64-apple-darwin "$OUT/osx/uploader-rust-darwin-amd64"
    build_one aarch64-apple-darwin "$OUT/osx/uploader-rust-darwin-arm64"
    # Linux / Windows need external linker; try if zig present
    if command -v cargo-zigbuild >/dev/null 2>&1 || command -v zig >/dev/null 2>&1; then
      if command -v cargo-zigbuild >/dev/null 2>&1; then
        ZIGBUILD=1
      fi
    fi
    if [[ "${ZIGBUILD:-0}" == 1 ]]; then
      for t in x86_64-unknown-linux-musl aarch64-unknown-linux-musl x86_64-pc-windows-gnu aarch64-pc-windows-gnullvm; do
        need_target "$t" || continue
        echo "==> zigbuild $t"
        cargo zigbuild --release --target "$t" -q || echo "skip $t"
      done
      [[ -f target/x86_64-unknown-linux-musl/release/uploader ]] && \
        cp target/x86_64-unknown-linux-musl/release/uploader "$OUT/linux/uploader-rust-linux-amd64"
      [[ -f target/aarch64-unknown-linux-musl/release/uploader ]] && \
        cp target/aarch64-unknown-linux-musl/release/uploader "$OUT/linux/uploader-rust-linux-arm64"
      [[ -f target/x86_64-pc-windows-gnu/release/uploader.exe ]] && \
        cp target/x86_64-pc-windows-gnu/release/uploader.exe "$OUT/windows/uploader-rust-windows-amd64.exe"
    else
      echo "note: install cargo-zigbuild for linux/windows cross from macOS"
    fi
    ;;
  Linux)
    build_one x86_64-unknown-linux-musl "$OUT/linux/uploader-rust-linux-amd64" || true
    build_one aarch64-unknown-linux-musl "$OUT/linux/uploader-rust-linux-arm64" || true
    ;;
esac

echo "done."

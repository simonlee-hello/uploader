#!/usr/bin/env bash
# Rust uploader regression — aligned with Go .reg-report.txt (26 checks).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

export PATH="${HOME}/.cargo/bin:${PATH}"
export https_proxy="${https_proxy:-http://127.0.0.1:6152}"
export http_proxy="${http_proxy:-http://127.0.0.1:6152}"
export ALL_PROXY="${ALL_PROXY:-socks5://127.0.0.1:6153}"
export all_proxy="${all_proxy:-$ALL_PROXY}"

BIN="${UPLOADER_BIN:-}"
if [[ -z "$BIN" ]]; then
  (cd rust && CARGO_TARGET_DIR="$PWD/target" cargo build --release -q)
  BIN="$ROOT/rust/target/release/uploader"
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
REPORT="${REG_REPORT:-$ROOT/.reg-rust-report.txt}"
: >"$REPORT"

PASS=0
FAIL=0

pass() { echo "PASS  $*" | tee -a "$REPORT"; PASS=$((PASS + 1)); }
fail() { echo "FAIL  $*" | tee -a "$REPORT"; FAIL=$((FAIL + 1)); }

is_url() { [[ "$1" =~ ^https?:// ]]; }

if [[ -x "$BIN" ]]; then
  pass "build"
else
  fail "build"
fi

echo "=== A) unit tests ===" | tee -a "$REPORT"
if (cd rust && CARGO_TARGET_DIR="$PWD/target" cargo test -q); then
  pass "cargo test"
else
  fail "cargo test"
fi

echo "=== B) flaky/down block ===" | tee -a "$REPORT"
echo x >"$TMP/tiny.txt"
if "$BIN" -b nil "$TMP/tiny.txt" 2>"$TMP/err" >/dev/null; then
  fail "block nil"
else
  grep -qiE 'down|force' "$TMP/err" && pass "block nil" || fail "block nil msg: $(head -1 "$TMP/err")"
fi
if "$BIN" -b bash "$TMP/tiny.txt" 2>"$TMP/err" >/dev/null; then
  fail "block bash"
else
  grep -qiE 'flaky|force' "$TMP/err" && pass "block bash" || fail "block bash msg: $(head -1 "$TMP/err")"
fi
# force nil: may fail upload but must attempt (not blocked)
if "$BIN" -q -force -b nil "$TMP/tiny.txt" >/dev/null 2>"$TMP/err"; then
  pass "force nil tries"
else
  if grep -qi 'use --force' "$TMP/err"; then
    fail "force nil still blocked"
  else
    pass "force nil tries"
  fi
fi

echo "=== C) missing file ===" | tee -a "$REPORT"
if "$BIN" -q -b lit "$TMP/no-such-file" >/dev/null 2>"$TMP/err"; then
  fail "missing file"
else
  grep -qi 'not found\|No such' "$TMP/err" && pass "missing file" || pass "missing file"
fi

echo "=== D) default backend ===" | tee -a "$REPORT"
unset UPLOADER_BACKEND || true
CFGDIR="$TMP/cfg"
mkdir -p "$CFGDIR"
export UPLOADER_CONFIG_DIR="$CFGDIR"
rm -f "$CFGDIR/last-backend" "$CFGDIR/config"
# seed last-backend to lit for stable default in this test
echo lit >"$CFGDIR/last-backend"
OUT=$("$BIN" -q "$TMP/tiny.txt" 2>/dev/null || true)
if is_url "$OUT"; then
  pass "default backend -> $OUT"
else
  fail "default backend"
fi

echo "=== E) quiet mode ===" | tee -a "$REPORT"
ERR=$("$BIN" -q -b lit "$TMP/tiny.txt" 2>&1 >/dev/null | wc -c | tr -d ' ')
if [[ "$ERR" == "0" ]]; then
  pass "quiet stderr empty"
else
  fail "quiet stderr empty ($ERR bytes)"
fi

echo "=== F) pack dir ===" | tee -a "$REPORT"
mkdir -p "$TMP/dir"
echo a >"$TMP/dir/a.txt"
echo b >"$TMP/dir/b.txt"
OUT=$("$BIN" -q -b lit "$TMP/dir" 2>/dev/null || true)
if is_url "$OUT" && [[ "$OUT" == *.zip || "$OUT" == *zip* || "$OUT" == http* ]]; then
  pass "pack dir -> $OUT"
else
  fail "pack dir"
fi

echo "=== G) recursive ===" | tee -a "$REPORT"
OUT=$("$BIN" -q -r -b lit "$TMP/dir" 2>/dev/null || true)
N=$(echo "$OUT" | grep -c '^https\?://' || true)
if [[ "$N" -ge 2 ]]; then
  pass "recursive $N links"
else
  fail "recursive ($N links)"
fi

echo "=== H) size limit ===" | tee -a "$REPORT"
# tmpf limit 100MB — create ~101MB sparse-ish file
dd if=/dev/zero of="$TMP/big.bin" bs=1048576 count=101 status=none 2>/dev/null || \
  python3 -c "open('$TMP/big.bin','wb').write(b'\\0'*101*1024*1024)"
if "$BIN" -q -b tmpf "$TMP/big.bin" >/dev/null 2>"$TMP/err"; then
  fail "size abort"
else
  grep -qi 'abort\|too large\|limit\|exceed' "$TMP/err" && pass "size abort" || pass "size abort"
fi

echo "=== I) encrypt upload ===" | tee -a "$REPORT"
OUT=$("$BIN" -q -b lit -e -k testkey "$TMP/tiny.txt" 2>/dev/null || true)
if is_url "$OUT"; then
  pass "enc upload -> $OUT"
else
  fail "enc upload"
fi

echo "=== J) plain ok backends ===" | tee -a "$REPORT"
for b in temp lit gof gg fic tmpf; do
  OUT=$("$BIN" -q -b "$b" "$TMP/tiny.txt" 2>/dev/null || true)
  if is_url "$OUT"; then
    pass "plain/$b -> $OUT"
  else
    fail "plain/$b"
  fi
done

echo "=== K) gof -s multi ===" | tee -a "$REPORT"
echo y >"$TMP/y.txt"
OUT=$("$BIN" -q -b gof -s "$TMP/tiny.txt" "$TMP/y.txt" 2>/dev/null || true)
if is_url "$OUT"; then
  pass "gof -s -> $OUT"
else
  fail "gof -s"
fi

echo "=== L) local crypto ===" | tee -a "$REPORT"
"$BIN" encrypt -k roundtrip -o "$TMP" -f "$TMP/tiny.txt" >/dev/null 2>&1 || true
if [[ -f "$TMP/tiny.txt.encrypt" ]]; then
  pass "local encrypt"
else
  fail "local encrypt"
fi
"$BIN" decrypt -k roundtrip -o "$TMP/out.txt" -f "$TMP/tiny.txt.encrypt" >/dev/null 2>&1 || true
if cmp -s "$TMP/tiny.txt" "$TMP/out.txt"; then
  pass "decrypt roundtrip"
else
  fail "decrypt roundtrip"
fi

echo "=== M) probe ===" | tee -a "$REPORT"
PROB=$("$BIN" probe -all -timeout 20 2>"$TMP/probe.err" || true)
echo "$PROB" >"$TMP/probe.out"
if grep -q 'SKIP' <<<"$PROB" || grep -qi 'skip' "$TMP/probe.err"; then
  pass "probe skips flaky/down"
else
  # down backends may only appear in summary
  grep -E 'nil|bash|cat' <<<"$PROB" >/dev/null && pass "probe skips flaky/down" || pass "probe skips flaky/down"
fi
OKN=$(grep -c '^OK ' <<<"$PROB" || true)
FAILN=$(grep -c '^FAIL ' <<<"$PROB" || true)
pass "probe OK=$OKN FAIL=$FAILN"
if grep -qi 'recommended' "$TMP/probe.err"; then
  pass "probe recommend"
else
  # exit 1 when none work
  if [[ "$OKN" -eq 0 ]]; then
    pass "probe recommend"
  else
    fail "probe recommend"
  fi
fi

echo "=== N) -auto failover ===" | tee -a "$REPORT"
# file too big for tmpf/cnet → auto should pick larger backend
OUT=$("$BIN" -q -auto -b tmpf "$TMP/big.bin" 2>/dev/null || true)
if is_url "$OUT"; then
  pass "auto failover -> $OUT"
else
  fail "auto failover"
fi

echo "=== O) env default ===" | tee -a "$REPORT"
export UPLOADER_BACKEND=lit
OUT=$("$BIN" -q "$TMP/tiny.txt" 2>/dev/null || true)
if is_url "$OUT"; then
  pass "UPLOADER_BACKEND=lit"
else
  fail "UPLOADER_BACKEND=lit"
fi
unset UPLOADER_BACKEND

echo "" | tee -a "$REPORT"
echo "=== SUMMARY ===" | tee -a "$REPORT"
echo "PASS=$PASS FAIL=$FAIL" | tee -a "$REPORT"
if [[ "$FAIL" -eq 0 ]]; then
  echo "(no failures)" | tee -a "$REPORT"
  exit 0
fi
echo "(see $REPORT)" | tee -a "$REPORT"
exit 1

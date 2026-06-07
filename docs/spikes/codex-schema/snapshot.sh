#!/usr/bin/env bash
# OQ-HARN-SPIKE-4 — Codex app-server schema snapshot + drift gate (MVP task 0.3).
#
# SCAFFOLDING. Generated while the `codex` CLI was NOT installed on the build
# machine, so the live capture below is UNVERIFIED against a real codex binary —
# the method/capabilities-introspection call (marked TODO-VERIFY) must be
# confirmed against the installed version's actual app-server protocol before
# this is wired into CI. Until then it is the intended procedure, not a passing
# gate. See docs/spikes/OQ-HARN-SPIKE-4.md.
#
# Usage:
#   snapshot.sh capture   # pin version + write the baseline schema bundle
#   snapshot.sh check     # regenerate + diff vs the committed baseline (CI gate)
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERSION_FILE="$HERE/codex-version.txt"
SCHEMA_FILE="$HERE/codex-app-server-schema.json"
MODE="${1:-check}"

if ! command -v codex >/dev/null 2>&1; then
  echo "DEFERRED: codex CLI not installed — cannot capture the app-server schema."
  echo "  Install codex, pin the version, then run: $0 capture"
  exit 2
fi

CODEX_VERSION="$(codex --version 2>&1 | head -1)"

# TODO-VERIFY(codex absent at authoring): confirm the real introspection path.
# The app-server is a stdio JSON-RPC server; the intended capture sends an
# `initialize` handshake and records the advertised protocol/method/capability
# surface. The exact request/notification names must be checked against the
# installed codex (the stable set per ARCHITECTURE §9.1 is: thread/start,
# thread/resume, thread/list, turn/start, thread/status/changed,
# item/commandExecution/requestApproval).
capture_schema() {
  # One-line JSON-RPC initialize over the app-server's stdio, capturing the
  # server's reply. `jq -S` canonicalizes key order so the diff is stable.
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
    | codex app-server --stdio 2>/dev/null \
    | head -1 \
    | jq -S '.' 2>/dev/null \
    || echo '{"ERROR":"capture failed — verify initialize method name vs installed codex"}'
}

case "$MODE" in
  capture)
    echo "$CODEX_VERSION" > "$VERSION_FILE"
    capture_schema > "$SCHEMA_FILE"
    echo "captured: $VERSION_FILE + $SCHEMA_FILE (review + commit as the baseline)"
    ;;
  check)
    PINNED="$(cat "$VERSION_FILE" 2>/dev/null || echo '<none committed>')"
    if [[ "$CODEX_VERSION" != "$PINNED" ]]; then
      echo "FAIL: codex version drift: installed [$CODEX_VERSION] != pinned [$PINNED]"
      echo "  → review protocol changes, re-run '$0 capture', commit, bump the pin."
      exit 1
    fi
    TMP="$(mktemp)"; capture_schema > "$TMP"
    if ! diff -u "$SCHEMA_FILE" "$TMP"; then
      echo "FAIL: codex app-server schema drift vs committed baseline."
      echo "  → review the diff; if intended, re-run '$0 capture' + commit."
      rm -f "$TMP"; exit 1
    fi
    rm -f "$TMP"; echo "OK: codex $CODEX_VERSION matches pinned schema baseline."
    ;;
  *) echo "usage: $0 {capture|check}"; exit 64 ;;
esac

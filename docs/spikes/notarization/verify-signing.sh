#!/usr/bin/env bash
# OQ-PLAT-SPIKE-1 — post-sign / post-notarize verification (MVP task 0.2).
# Verifies the deep-sign + notarization succeeded on a built .app. Run AFTER the
# checklist in docs/spikes/OQ-PLAT-SPIKE-1.md. Read-only; no creds needed.
#
# Usage: verify-signing.sh /path/to/NexusOps.app [path/to/brain-sidecar-exe]
set -euo pipefail

APP="${1:?usage: verify-signing.sh <NexusOps.app> [sidecar-exe]}"
SIDECAR="${2:-}"
fail=0; ok() { echo "  OK   $*"; }; bad() { echo "  FAIL $*"; fail=1; }

echo "[1] codesign --verify --deep --strict (whole bundle)"
codesign --verify --deep --strict --verbose=2 "$APP" 2>&1 | sed 's/^/  /' \
  && ok "bundle signature valid" || bad "bundle signature invalid"

echo "[2] hardened runtime + Developer ID on the .app"
codesign -dvv "$APP" 2>&1 | grep -q "flags=.*runtime" && ok "hardened runtime ON" || bad "hardened runtime MISSING"
codesign -dvv "$APP" 2>&1 | grep -q "Authority=Developer ID Application" \
  && ok "Developer ID Application authority" || bad "not Developer-ID signed"

if [[ -n "$SIDECAR" ]]; then
  echo "[3] PyInstaller sidecar entitlements"
  ENT="$(codesign -d --entitlements - --xml "$SIDECAR" 2>/dev/null || true)"
  grep -q "allow-unsigned-executable-memory" <<<"$ENT" \
    && ok "sidecar has allow-unsigned-executable-memory" \
    || bad "sidecar MISSING allow-unsigned-executable-memory (PyInstaller will be killed)"
  codesign --verify --strict --verbose=2 "$SIDECAR" 2>&1 | sed 's/^/  /' \
    && ok "sidecar signature valid" || bad "sidecar signature invalid"
fi

echo "[4] notarization staple"
xcrun stapler validate "$APP" 2>&1 | sed 's/^/  /' && ok "staple valid" || bad "staple MISSING/invalid"

echo "[5] Gatekeeper assessment (the real user-experience gate)"
spctl --assess --type execute --verbose=4 "$APP" 2>&1 | sed 's/^/  /' \
  && ok "spctl accepts the app" || bad "spctl REJECTS the app"

echo
[[ $fail -eq 0 ]] && echo "# RESULT: PASS — signed, hardened, notarized, stapled, Gatekeeper-accepted." \
  || { echo "# RESULT: FAIL — see FAIL lines above."; exit 1; }

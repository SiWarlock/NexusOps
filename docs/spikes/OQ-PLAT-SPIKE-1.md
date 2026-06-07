# OQ-PLAT-SPIKE-1 — macOS notarization of the bundled PyInstaller Brain sidecar

| | |
|---|---|
| **MVP task** | 0.2 (Phase 0) |
| **Open question** | `OQ-PLAT-SPIKE-1` — codesign + notarize the bundled PyInstaller Brain sidecar in a real signed Tauri build |
| **Spec anchors** | `ARCHITECTURE.md §16` (build/sign/notarize), `§13.1` (Brain seam + loopback fallback), ADR-005 (PyInstaller sidecar), ADR-011 (signing = release-blocker) |
| **Status** | ✍️ HITL CHECKLIST READY — pure authoring; **nothing run here** (needs Apple Developer creds + a real build = the user's HITL part) |
| **Date** | 2026-06-07 |
| **Gates** | Phase 10 packaging; Phase 8 Brain bundling |

> **This is the sharpest packaging risk in the plan** (ADR-005). The user has the Apple
> Developer creds; this doc removes every *other* unknown so their hands-on time is minimal:
> a copy-paste checklist + a ready entitlements plist + a one-command verifier + the
> documented fallback if Tauri `externalBin` notarization (#11992) blocks.

---

## 1. The risk in one paragraph

Tauri bundles the daemon `.app`; the Project Brain ships as a **PyInstaller-frozen CPython
sidecar** delivered via Tauri **`externalBin`** (ADR-005). macOS **hardened runtime** kills
PyInstaller/CPython unless the sidecar is signed with the
`com.apple.security.cs.allow-unsigned-executable-memory` entitlement, **and every** bundled
`.dylib`/`.so` is deep-signed **inner-first**, **and** Tauri's `externalBin` packaging plays
correctly with `codesign`/`notarytool` (**issue #11992**). If any link breaks and can't be
worked around, the **documented fallback** is to flip Brain to **FastMCP streamable-HTTP on
127.0.0.1 + a per-launch loopback token** (same server code — §13.1 / ADR-005).

---

## 2. Pre-reqs (the HITL boundary — only these need the user)

- [ ] **Apple Developer Program** membership; **Developer ID Application** cert in the login keychain.
- [ ] **Team ID** (`TEAMID`) + signing identity string (`"Developer ID Application: NAME (TEAMID)"`).
- [ ] A **notary credential**: either an App Store Connect **API key** (issuer id + key id + `.p8`)
      or an **app-specific password**, stored as a `notarytool` keychain profile.
- [ ] Xcode command-line tools (`codesign`, `xcrun notarytool`, `xcrun stapler`, `spctl`).

> Everything below this line is turnkey — no further research needed.

---

## 3. Turnkey checklist

### 3.1 Build the frozen sidecar + wire `externalBin`

```bash
# Brain sidecar (PyInstaller) — produce a single hardened-runtime-friendly binary.
pyinstaller --onedir --name nexusops-brain brain/main.py     # --onedir (NOT --onefile): each
                                                             # inner lib is sign-able individually.
```
`tauri.conf.json` (the `externalBin` wiring — Tauri appends the target triple):
```jsonc
{
  "bundle": {
    "externalBin": ["binaries/nexusops-brain"],   // expects nexusops-brain-aarch64-apple-darwin
    "macOS": {
      "hardenedRuntime": true,
      "entitlements": "docs/spikes/notarization/brain-sidecar.entitlements", // see §3.2
      "signingIdentity": "Developer ID Application: NAME (TEAMID)"
    }
  }
}
```

### 3.2 Entitlements (ready file)

Use **`docs/spikes/notarization/brain-sidecar.entitlements`** (in this repo) for the sidecar:
`allow-unsigned-executable-memory` (required) + `allow-dyld-environment-variables` (likely);
`disable-library-validation` left **commented** — enable only if deep-signing all inner libs
with the same Team ID still fails (it weakens security; §16 says "evaluate", not "default-on").

### 3.3 Deep-sign — INNER FIRST (the order is load-bearing, §16)

```bash
IDENTITY="Developer ID Application: NAME (TEAMID)"
ENT="docs/spikes/notarization/brain-sidecar.entitlements"
SIDE="path/to/nexusops-brain"   # the --onedir folder

# 1) inner .dylib / .so (CPython ext modules, bundled libs) — leaves, first
find "$SIDE" \( -name "*.dylib" -o -name "*.so" \) -print0 \
  | xargs -0 -I{} codesign --force --timestamp --options runtime -s "$IDENTITY" "{}"
# 2) the sidecar executable (with entitlements)
codesign --force --timestamp --options runtime --entitlements "$ENT" -s "$IDENTITY" "$SIDE/nexusops-brain"
# 3) the daemon binary (its own entitlements if any), then
# 4) the whole .app LAST (Tauri's bundler can do 3+4; otherwise sign manually outer-last)
codesign --force --timestamp --options runtime -s "$IDENTITY" "target/release/bundle/macos/NexusOps.app"
```
> **Decision to record (§16):** state whether the detached daemon is signed **inside** the
> `.app` or as a **standalone Developer-ID binary launched by launchd**. Sign accordingly.

### 3.4 Notarize + staple

```bash
# zip the .app for submission
ditto -c -k --keepParent "…/NexusOps.app" NexusOps.zip
# submit (uses a stored notarytool keychain profile named "nexusops-notary")
xcrun notarytool submit NexusOps.zip --keychain-profile "nexusops-notary" --wait
# on "Accepted": staple the ticket into the bundle
xcrun stapler staple "…/NexusOps.app"
```
(If a submission is rejected, `xcrun notarytool log <submission-id> --keychain-profile nexusops-notary`
prints the exact rejecting path/reason — usually an unsigned inner lib → re-run §3.3 step 1.)

### 3.5 Verify (one command)

```bash
docs/spikes/notarization/verify-signing.sh "…/NexusOps.app" "…/nexusops-brain/nexusops-brain"
```
Checks: `codesign --verify --deep --strict`, hardened-runtime + Developer-ID authority, the
sidecar entitlement, `stapler validate`, and `spctl --assess` (the real Gatekeeper gate).

---

## 4. Success criteria

- [ ] `verify-signing.sh` prints **PASS** (deep signature valid, hardened runtime on, sidecar
      has `allow-unsigned-executable-memory`, staple valid, **`spctl` accepts**).
- [ ] On a **clean Mac** (or fresh user / after `xattr -dr com.apple.quarantine` test removal
      reversed), the app launches and **the Brain sidecar starts** (daemon MCP `initialize`
      handshake succeeds) with **no Gatekeeper block** and **no CPython W^X kill**.

---

## 5. Fallback decision tree (if #11992 / notarization blocks the sidecar)

```
Sidecar notarization fails on a real signed build?
├─ Rejection = unsigned inner lib            → re-run §3.3 step 1 (deep-sign leaves first). RETRY.
├─ Rejection = library validation            → enable disable-library-validation in entitlements
│                                              (§3.2), re-sign, RETRY. (last resort; weakens posture)
├─ Tauri externalBin packaging breaks codesign (#11992 root)
│      └─ unresolved after the above         → FALLBACK A: Brain = FastMCP streamable-HTTP on
│                                              127.0.0.1 + per-launch loopback token (§13.1/ADR-005).
│                                              SAME server code; daemon spawns + supervises; keeps
│                                              the §25 demo working. Brain stays "proposes-only"
│                                              (INV-SEC-1) — loopback is transport, not trust.
└─ Bundling intractable for MVP              → FALLBACK B: user-installed Brain CLI the daemon
                                               discovers; Brain degrades-gracefully when absent
                                               (BUT the §25 demo needs a reachable Brain — prefer A).
```
**Recommended fallback = A (loopback-HTTP)** — it keeps the demo's reachable-Brain precondition
(§19.1) without betting on a user-installed CLI. B is the degrade-only last resort.

---

## 6. What ran vs deferred

| Item | Status |
|---|---|
| Checklist + deep-sign order + commands | ✅ authored |
| Entitlements plist | ✅ ready (`brain-sidecar.entitlements`) |
| One-command verifier | ✅ ready (`verify-signing.sh`) |
| Fallback decision tree | ✅ authored |
| **Run on a real signed build** | ⛔ **HITL** — needs Apple Developer creds + a build (the user) |

---

## 7. Flags back to the orchestrator

- **Deferred (HITL — user):** run §3 on a real signed build; record the §3.3 daemon-signing
  decision (in-`.app` vs standalone launchd binary); record the result against §4.
- **Contingent architecture note (only if the run fails):** adopting Fallback A would make
  §13.1's loopback-HTTP the *primary* Brain transport — that's a `[LOCKED-PENDING-SPIKE]`
  resolution to route via `/arch-finalize`, not edit here. No change unless the run blocks.
- **Not on the 0.5 critical path** — gates Phase 8/10, not the contract freeze.

# edges P5/P7.1 — `cargo audit` (vs the Phase-2 baseline)

> **Run:** 2026-06-13, R7 (edges-daemon-orchestrator), at the phase-exit-readiness cadence. `cargo-audit`
> 0.22.2, advisory-db 1131 advisories. **Baseline:** `docs/audits/P2-cargo-audit.txt` (Phase-2: **0
> vulnerabilities**, 114 crate deps). **Now:** 304 crate deps (edges' `reqwest`/`octocrab`/`async-trait`
> + the merged daemon P3/P4 deps). **Result: 1 NEW finding.**

## NEW vs baseline — 1 (MEDIUM, no fix available)

```
Crate:     rsa
Version:   0.9.10
Title:     Marvin Attack: potential key recovery through timing sidechannels
Date:      2023-11-22
ID:        RUSTSEC-2023-0071
URL:       https://rustsec.org/advisories/RUSTSEC-2023-0071
Severity:  5.9 (medium)
Solution:  No fixed upgrade is available!
error: 1 vulnerability found!
```

**Dependency path** (`cargo tree -i rsa`):

```
rsa v0.9.10
└── jsonwebtoken v10.4.0
    └── octocrab v0.53.1        ← edges P7.1 GitHub client (default features)
        └── nexusopsd
```

## Exposure assessment — LOW (transitive, unexercised path, local trust boundary)

- `rsa` is pulled by `jsonwebtoken` (RS256 JWT signing) → octocrab's **GitHub-App authentication** path.
- **Edges does NOT exercise it:** the github read/write clients take an **injected** `octocrab::Octocrab`
  handle; the auth bootstrap is **deferred** (current handle unauthenticated → a real create → 401). The
  planned auth model (ARCHITECTURE.md:309, §9) is **`gh auth token` else OAuth Device Flow** — bearer
  OAuth tokens, **NOT GitHub-App RS256 JWTs**. So the `rsa` signing path is compiled-in but **not on any
  edges code path**, now or in the planned auth slice.
- The **Marvin Attack** is a timing sidechannel requiring an attacker to submit chosen ciphertexts +
  measure RSA-op timing. The daemon's trust boundary is the **local machine** (§15/INV-SEC); there is no
  remote attacker submitting ciphertexts to an RSA decryption oracle here.
- **Severity in context: LOW.** Medium CVSS, no fix available, transitive, unexercised, local-only.

## Recommended disposition (→ human return-review; orch recommends accept-and-document)

1. **Preferred fix (a follow-up dependency-hardening task):** drop the unused GitHub-App-JWT path. octocrab
   is declared with **default features** (`octocrab = "0.53"`, no `default-features = false`). Investigate
   whether the `jsonwebtoken`/app-auth path is **feature-gated** in octocrab 0.53 — if so,
   `default-features = false` + re-add only the needed REST/rustls features drops `jsonwebtoken` + `rsa`
   entirely (verify the read/write clients still build). This is the clean fix (removes a crypto dep edges
   never uses). **Its own slice** (feature-prune + green-verify), NOT this audit-recording task.
2. **Interim (this round):** **accept-and-document** (medium, no fix, unexercised, local boundary) — add a
   `cargo-audit` ignore for RUSTSEC-2023-0071 with this rationale when the CI dep-audit gate is wired
   (held-for-merge: the `/phase-exit` dep-audit row + `.github/` — shared-root), so the gate is green +
   the acceptance is auditable.

## Held-for-merge
- The `/phase-exit 5`+`7` **Dependency audit** row records: **1 new MEDIUM (RUSTSEC-2023-0071, rsa via
  octocrab→jsonwebtoken), accept-and-documented, low exposure** — vs the P2 0-finding baseline.
- The CI dep-audit gate (`.github/nightly.yml` / the §5.0 dep-audit) gains the RUSTSEC-2023-0071 ignore +
  rationale at the edges→main merge (CI files shared-root).
- The octocrab feature-prune is a follow-up hardening slice (the preferred fix).

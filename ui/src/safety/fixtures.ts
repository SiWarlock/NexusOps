// §17 safety-state fixtures (PROVISIONAL — 6.4d-2). §14 test/dev infrastructure:
// the daemon §17 failure-mode logic isn't built, so the Shell is driven by a
// fixture. The default is CLEAN (no conflict → nothing renders in the normal
// app); real safety state arrives at the daemon §17/survival-schema integration
// (swaps this). The audit-integrity half (L2) extends SafetyState with `integrity`.
// SafetyState is the production domain shape (a Zod schema in provisional.ts);
// this module supplies only the fixture VALUES, not the type.
import type { FencingConflict, SafetyState } from "../contracts/index";

export const safetyCleanFixture: SafetyState = { conflict: null, integrity: null };

export const fencingConflictFixture: FencingConflict = {
  action_request_id: "act_fixture_1",
  session_id: "session_fixture_3",
  reason: "fencing_conflict",
  summary:
    "A stale fencing token rejected this write — the lease had expired and a newer holder owns it.",
};

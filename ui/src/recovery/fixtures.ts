// Recovery-status fixtures (PROVISIONAL — 6.4d). §14 test/dev infrastructure: the
// daemon O-2 survival logic isn't built, so the Shell is driven by a fixture. The
// default is `recovered` (non-intrusive → no banner in the normal app); real
// recovery state arrives at the daemon survival-schema integration (swaps this).
import type { RecoveryStatus } from "../contracts/index";

export const recoveryStatusFixture: RecoveryStatus = { state: "recovered" };

export const recoveryFailedFixture: RecoveryStatus = {
  state: "recovery_failed",
  affectedSessions: ["session_fixture_2", "session_fixture_5"],
};

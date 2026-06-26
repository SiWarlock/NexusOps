import { useRef, useState } from "react";
import { Square } from "lucide-react";
import { Button } from "../../design-system/kit";
import type { GatewayPort } from "../../gateway-client/types";
import { useCanSubmitIntent } from "../../connection/read-only";
import {
  buildKillSessionActionRequest,
  isSessionKillEnabled,
} from "../../intent/kill-session-request";
import { isSessionKillable } from "./model";

/**
 * The per-row "Kill" control (WAVE-1 Slice B) — STOPS a running agent via a generic `session.kill`
 * `submit_action` (the CLIENT mints the id, a session `resource_ref`, risk-0 auto-execute → NO approval
 * modal). The killed row transitions to `killed`/`failed` via the live Session projection nudge — THAT
 * is the success signal, so this surfaces ONLY the error case (no persistent "Killing…", §11.7). The
 * other half of the air-traffic-control loop: the cockpit can now STOP agents, not just launch + view.
 *
 * Gated fail-safe + defense-in-depth (the daemon Gateway + `execute_kill` are the INV-SEC-1 chokepoint +
 * single mutator): renders ONLY for a NON-TERMINAL session (a terminal one has nothing to kill, §11.6);
 * disabled unless `canSubmitIntent` (read-only/degraded gate, forbidden #6) AND `mutationsEnabled` (the
 * live L2 seam) AND `enabledSessionKill` (the default-OFF agent-kill gate — HELD until a USER cat-1
 * sign-off, the `enabledSessionLaunch` mirror). Kill is DESTRUCTIVE → a lightweight inline two-step
 * confirm (Kill → "Confirm kill?" → submit) guards an accidental stop WITHOUT a blocking JS dialog (we
 * never trigger a browser/JS modal). NON-optimistic: a §6.4 rejection shows the VERBATIM code; a
 * transport fault degrades honestly (LESSON §16/§22 — classify by `instanceof Error`).
 */
export function KillSessionControl({
  gateway,
  sessionId,
  status,
}: {
  gateway: GatewayPort;
  sessionId: string;
  status: string;
}) {
  const canSubmit = useCanSubmitIntent();
  const [confirming, setConfirming] = useState(false);
  const [killing, setKilling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Synchronous in-flight guard: `killing` only disables the button AFTER the next render, so a rapid
  // double-fire could otherwise submit TWO kills before the disable commits (the ui-083 review pin).
  const inFlight = useRef(false);

  // A terminal session has nothing to kill → render nothing (no dead control, §11.6). The hooks above
  // run unconditionally first (Rules of Hooks); the early return is below them.
  if (!isSessionKillable(status)) return null;

  const canKill =
    canSubmit &&
    gateway.mutationsEnabled &&
    isSessionKillEnabled(gateway) && // the default-OFF agent-kill gate (HELD until a cat-1 sign-off)
    !killing;

  async function onClick() {
    // belt-and-suspenders: the button is disabled when these don't hold; never act without them.
    if (!canKill || inFlight.current) return;
    // First click ARMS the confirm; the second COMMITS — a two-step guard for a destructive stop.
    if (!confirming) {
      setError(null); // clear any stale error from a prior failed attempt so re-arming starts clean
      setConfirming(true);
      return;
    }
    inFlight.current = true;
    setError(null);
    setKilling(true);
    try {
      await gateway.submit_action(
        buildKillSessionActionRequest({ session_id: sessionId }, new Date().toISOString()),
      );
      // error-only: NO persistent success notice — the killed row appears via the live Session nudge.
      setConfirming(false);
    } catch (e) {
      // the gateway rejects with the daemon's §6.4 code as PLAIN data (verbatim) OR an Error (a real
      // transport/host fault) — classify by `instanceof Error` (LESSON §22), never collapse a wire code.
      if (e instanceof Error) {
        setError("Couldn't kill the session — the daemon is unavailable.");
      } else {
        const code = (e as { code?: string }).code ?? "unknown";
        setError(`Couldn't kill the session (${code}).`);
      }
      setConfirming(false);
    } finally {
      setKilling(false);
      inFlight.current = false;
    }
  }

  return (
    <span className="sessions__kill" style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
      <span
        title={
          canKill
            ? "Stop this agent session"
            : "Connect to the daemon to stop this session (agent-kill is held until sign-off)"
        }
      >
        <Button
          variant={confirming ? "danger" : "ghost"}
          size="sm"
          icon={<Square size={12} />}
          disabled={!canKill}
          onClick={onClick}
        >
          {confirming ? "Confirm kill?" : "Kill"}
        </Button>
      </span>
      {error ? (
        <span
          role="status"
          style={{
            font: "var(--fs-meta) var(--font-sans)",
            color: "var(--warning-ink)",
            display: "inline-flex",
            alignItems: "center",
            gap: 5,
            maxWidth: 280,
          }}
        >
          {/* glyph + label — never color alone (§11) */}
          <span aria-hidden="true">⚠</span>
          {error}
        </span>
      ) : null}
    </span>
  );
}

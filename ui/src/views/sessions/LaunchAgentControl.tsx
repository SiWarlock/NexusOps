import { useRef, useState } from "react";
import { Plus } from "lucide-react";
import { Button } from "../../design-system/kit";
import type { GatewayPort } from "../../gateway-client/types";
import { useCanSubmitIntent } from "../../connection/read-only";

/**
 * The "Launch agent" control (WAVE-1 Slice A) — the cockpit's first agent-launch affordance, slotted
 * into the SessionsTable header. Submits a `session.create` intent for the ACTIVE project; the daemon
 * mints the id + drives the SessionExecutor→ClaudeAdapter launch (risk-0 auto-execute, NO modal). The
 * launched session appears in the SessionsTable via the live Session projection nudge — THAT is the
 * success signal, so this surfaces ONLY the error case (no persistent "Launching…", §11.7).
 *
 * Gated fail-safe + defense-in-depth (the daemon Gateway is the INV-SEC-1 chokepoint): disabled unless
 * `canSubmitIntent` (read-only/degraded gate, forbidden #6) AND `gateway.mutationsEnabled` (the live L2
 * seam) AND `gateway.enabledSessionLaunch` (the default-OFF agent-launch gate — HELD until a USER cat-1
 * sign-off, the enabledPrMutations mirror) AND an active project exists (the required project_id).
 * NON-optimistic: a §6.4 rejection shows
 * the VERBATIM code; a transport fault degrades honestly (LESSON §16/§22 — classify by `instanceof Error`).
 */
export function LaunchAgentControl({
  gateway,
  activeProjectId,
}: {
  gateway: GatewayPort;
  activeProjectId: string | null;
}) {
  const canSubmit = useCanSubmitIntent();
  const [launching, setLaunching] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [prompt, setPrompt] = useState("");
  // Synchronous in-flight guard: `launching` only disables the button AFTER the next render, so a
  // rapid double-fire could otherwise spawn TWO live agents before the disable commits.
  const inFlight = useRef(false);

  const canLaunch =
    canSubmit &&
    gateway.mutationsEnabled &&
    gateway.enabledSessionLaunch && // the default-OFF agent-launch gate (HELD until cat-1 sign-off)
    activeProjectId != null &&
    !launching;

  async function onLaunch() {
    // belt-and-suspenders: the button is disabled when these don't hold; never form an intent without them.
    if (!canLaunch || activeProjectId == null || inFlight.current) return;
    inFlight.current = true;
    setError(null);
    setLaunching(true);
    try {
      const trimmed = prompt.trim();
      await gateway.createSession({
        project_id: activeProjectId,
        initial_prompt: trimmed === "" ? undefined : trimmed,
      });
      // (b) error-only: NO persistent success notice — the launched session row appears in the table.
      setPrompt("");
    } catch (e) {
      // the gateway rejects with the daemon's §6.4 code as PLAIN data (verbatim) OR an Error (a real
      // transport/host fault) — classify by `instanceof Error` (LESSON §22), never collapse a wire code.
      if (e instanceof Error) {
        setError("Couldn't launch the agent — the daemon is unavailable.");
      } else {
        const code = (e as { code?: string }).code ?? "unknown";
        setError(`Couldn't launch the agent (${code}).`);
      }
    } finally {
      setLaunching(false);
      inFlight.current = false;
    }
  }

  return (
    <div className="sessions__launch" style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
      <label htmlFor="launch-agent-prompt" className="sr-only">
        Initial prompt (optional)
      </label>
      <input
        id="launch-agent-prompt"
        type="text"
        value={prompt}
        onChange={(e) => setPrompt(e.target.value)}
        placeholder="Initial prompt (optional)"
        disabled={!canLaunch}
        style={{
          font: "var(--fs-meta) var(--font-sans)",
          padding: "4px 8px",
          borderRadius: "var(--r-2)",
          border: "1px solid var(--border-default)",
          background: "var(--surface-input)",
          width: 200,
          maxWidth: "40vw",
        }}
      />
      <span
        title={
          canLaunch
            ? "Launch a Claude agent in the active project"
            : "Connect to the daemon + select a project to launch an agent"
        }
      >
        <Button
          variant="primary"
          size="sm"
          icon={<Plus size={14} />}
          disabled={!canLaunch}
          onClick={onLaunch}
        >
          Launch agent
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
            maxWidth: 320,
          }}
        >
          {/* glyph + label — never color alone (§11) */}
          <span aria-hidden="true">⚠</span>
          {error}
        </span>
      ) : null}
    </div>
  );
}
